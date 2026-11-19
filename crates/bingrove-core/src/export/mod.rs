//! Writing a planned drawer to disk: one STL per distinct part, 3MF files with the parts laid
//! out per printer bed, optionally the assembled drawer as 3MF, and the project file.

pub mod pack;
pub mod stl;
pub mod threemf;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::geom::all_parts;
use crate::geom::kernel::GeomError;
use crate::mesh::TriMesh;
use crate::model::DrawerProject;
use crate::plan::{PartSpec, Plan};

/// Writes `mesh` as binary STL to `dir/name`, creating `dir` if needed.
pub fn save_stl(dir: &Path, name: &str, mesh: &TriMesh) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name);
    std::fs::write(&path, stl::write_binary(mesh, &format!("bingrove {name}")))?;
    Ok(path)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportOptions {
    pub stl: bool,
    /// 3MF per printer bed, plate pieces and bins on separate beds.
    pub beds: bool,
    /// The whole drawer assembled, to look at.
    pub assembly: bool,
    pub project_file: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions { stl: true, beds: true, assembly: false, project_file: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Part,
    /// The body that fills a part's engraving: printed with that part, in a second colour.
    Inlay,
    Bed,
    Assembly,
    Project,
}

/// A written file, relative to the output folder, and how often to print it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportedFile {
    pub path: String,
    pub kind: FileKind,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportReport {
    pub files: Vec<ExportedFile>,
    pub beds: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error(transparent)]
    Geom(#[from] GeomError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// What the project file holds: the project to reopen, and what came out of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub format: String,
    pub generator: String,
    pub project: DrawerProject,
    pub files: Vec<ExportedFile>,
}

pub const PROJECT_FORMAT: &str = "bingrove-drawer";
pub const PROJECT_SUFFIX: &str = ".gridfinity.json";

/// File-name-safe version of a drawer name.
pub fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            'ä' => 'a',
            'ö' => 'o',
            'ü' => 'u',
            'ß' => 's',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => '_',
        })
        .collect();
    let s = s.split('_').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("_");
    if s.is_empty() { "schublade".into() } else { s }
}

/// Generates and writes everything. `progress(done, total, what)` is called per part.
pub fn export(
    plan: &Plan,
    project: &DrawerProject,
    out: &Path,
    opts: ExportOptions,
    progress: &dyn Fn(usize, usize, &str),
) -> Result<ExportReport, ExportError> {
    std::fs::create_dir_all(out)?;
    let total = plan.parts.len();
    progress(0, total, "plate");
    let parts = all_parts(plan, &project.profile);
    let mut meshes = Vec::with_capacity(parts.len());
    let mut inlays = Vec::with_capacity(parts.len());
    for (i, part) in parts.iter().enumerate() {
        meshes.push(part.mesh()?);
        inlays.push(part.inlay_mesh()?);
        progress(i + 1, total, &part.name);
    }
    let mut files = Vec::new();
    if opts.stl {
        for ((entry, mesh), inlay) in plan.parts.iter().zip(&meshes).zip(&inlays) {
            save_stl(out, &entry.name, mesh)?;
            files.push(ExportedFile { path: entry.name.clone(), kind: FileKind::Part, quantity: entry.quantity });
            if let Some(inlay) = inlay {
                let name = crate::names::inlay(&entry.name);
                save_stl(out, &name, inlay)?;
                files.push(ExportedFile { path: name, kind: FileKind::Inlay, quantity: 0 });
            }
        }
    }
    let slug = slug(&plan.name);
    let objects: Vec<threemf::Object> = plan
        .parts
        .iter()
        .zip(&meshes)
        .zip(&inlays)
        .map(|((e, m), inlay)| threemf::Object { name: &e.name, mesh: m, inlay: inlay.as_ref() })
        .collect();
    let mut beds = 0;
    if opts.beds {
        let dir = out.join("Druckbetten");
        std::fs::create_dir_all(&dir)?;
        // Every copy of every part, plate pieces in their own group.
        let mut groups: [Vec<usize>; 2] = [vec![], vec![]];
        for (i, e) in plan.parts.iter().enumerate() {
            let g = usize::from(!matches!(e.part, PartSpec::Plate { .. }));
            groups[g].extend(std::iter::repeat_n(i, e.quantity as usize));
        }
        for group in groups.iter().filter(|g| !g.is_empty()) {
            let sizes: Vec<[f64; 2]> = group.iter().map(|&i| [plan.parts[i].size[0], plan.parts[i].size[1]]).collect();
            let spots = pack::pack(&sizes, &project.bed);
            let count = spots.iter().map(|s| s.bed).max().map_or(0, |m| m + 1);
            for b in 0..count {
                let items: Vec<threemf::Item> = group
                    .iter()
                    .zip(&spots)
                    .filter(|(_, s)| s.bed == b)
                    .map(|(&i, s)| {
                        // A quarter turn puts the part left of x = 0; shift it back by its depth.
                        let dx = if s.turned { plan.parts[i].size[1] } else { 0.0 };
                        threemf::Item { object: i, quarter_turns: u8::from(s.turned), at: [s.at[0] + dx, s.at[1], 0.0] }
                    })
                    .collect();
                beds += 1;
                let name = format!("Druckbetten/{slug}_bett_{beds:02}.3mf");
                std::fs::write(
                    out.join(&name),
                    threemf::write(&format!("{} – Bett {beds}", plan.name), &objects, &items)?,
                )?;
                files.push(ExportedFile { path: name, kind: FileKind::Bed, quantity: 1 });
            }
        }
    }
    if opts.assembly && plan.kind == crate::model::Kind::Wall {
        // A wall stands up, which turns about x; the assembly goes in as one mesh.
        let mesh = crate::geom::assemble(plan, &parts, &meshes);
        let whole = [threemf::Object { name: &plan.name, mesh: &mesh, inlay: None }];
        let item = threemf::Item { object: 0, quarter_turns: 0, at: [0.0; 3] };
        let name = format!("{slug}_zusammenbau.3mf");
        std::fs::write(out.join(&name), threemf::write(&format!("{} – Zusammenbau", plan.name), &whole, &[item])?)?;
        files.push(ExportedFile { path: name, kind: FileKind::Assembly, quantity: 0 });
    } else if opts.assembly {
        let mut items = Vec::new();
        // Pieces were moved to the origin by their own minimum corner; put them back.
        for (i, part) in parts.iter().enumerate() {
            if matches!(plan.parts[i].part, PartSpec::Plate { .. }) {
                items.push(threemf::Item { object: i, quarter_turns: 0, at: [part.origin[0], part.origin[1], 0.0] });
            }
        }
        for p in &plan.placements {
            let [x, y, z] = plan.position(p);
            let size = plan.parts[p.part].size;
            let at = if p.turned { [x + size[0], y + size[1], z] } else { [x, y, z] };
            items.push(threemf::Item { object: p.part, quarter_turns: if p.turned { 2 } else { 0 }, at });
        }
        let name = format!("{slug}_zusammenbau.3mf");
        std::fs::write(out.join(&name), threemf::write(&format!("{} – Zusammenbau", plan.name), &objects, &items)?)?;
        files.push(ExportedFile { path: name, kind: FileKind::Assembly, quantity: 0 });
    }
    if opts.project_file {
        let name = format!("{slug}{PROJECT_SUFFIX}");
        files.push(ExportedFile { path: name.clone(), kind: FileKind::Project, quantity: 0 });
        let pf = ProjectFile {
            format: PROJECT_FORMAT.into(),
            generator: format!("bingrove {}", env!("CARGO_PKG_VERSION")),
            project: DrawerProject {
                placements: plan.placements.iter().map(|p| p.placement.clone()).collect(),
                ..project.clone()
            },
            files: files.clone(),
        };
        std::fs::write(out.join(&name), serde_json::to_string_pretty(&pf)?)?;
    }
    Ok(ExportReport { files, beds })
}

/// The print list as text, for files that leave the library: every part with how often to
/// print it and its size, the text bodies with the part they belong to, then the beds.
pub fn print_list(plan: &Plan, report: &ExportReport) -> String {
    use std::fmt::Write as _;
    let mut x = format!("Bingrove – {}\n\nTeile ({} Stück zu drucken):\n", plan.name, plan.part_count());
    for f in &report.files {
        match f.kind {
            FileKind::Part => {
                let size = plan.parts.iter().find(|p| p.name == f.path).map(|p| p.size).unwrap_or_default();
                let _ = writeln!(
                    x,
                    "  {:>3} ×  {}  ({:.1} × {:.1} × {:.1} mm)",
                    f.quantity, f.path, size[0], size[1], size[2]
                );
            }
            FileKind::Inlay => {
                let part = f.path.strip_suffix("_schrift.stl").map(|s| format!("{s}.stl")).unwrap_or_default();
                let _ =
                    writeln!(x, "        {}  (Schrift zu {part}: als Teil desselben Objekts in zweiter Farbe)", f.path);
            }
            _ => {}
        }
    }
    let beds: Vec<&ExportedFile> = report.files.iter().filter(|f| f.kind == FileKind::Bed).collect();
    if !beds.is_empty() {
        let _ = writeln!(x, "\nDruckbetten ({}, Teile fertig angeordnet):", beds.len());
        for f in beds {
            let _ = writeln!(x, "    1 ×  {}", f.path);
        }
    }
    x
}

/// Everything `export` wrote to `out`, as one ZIP: the files under `top/` with the paths they
/// have below `out`, and `top/druckliste.txt`.
pub fn zip(out: &Path, plan: &Plan, report: &ExportReport, top: &str) -> Result<Vec<u8>, ExportError> {
    use std::io::Write as _;
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut add = |name: &str, bytes: &[u8]| -> std::io::Result<()> {
        zip.start_file(format!("{top}/{name}"), stored).map_err(std::io::Error::other)?;
        zip.write_all(bytes)
    };
    for f in &report.files {
        add(&f.path, &std::fs::read(out.join(&f.path))?)?;
    }
    add("druckliste.txt", print_list(plan, report).as_bytes())?;
    Ok(zip.finish().map_err(std::io::Error::other)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("Kleinkram-Schublade 250×305"), "kleinkram_schublade_250_305");
        assert_eq!(slug("Küche/Gewürze"), "kuche_gewurze");
        assert_eq!(slug("!!!"), "schublade");
    }
}
