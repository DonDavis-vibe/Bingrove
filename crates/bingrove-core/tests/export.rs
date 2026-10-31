//! A whole drawer written to a folder: STLs with quantities, beds, assembly and project file
//! that reopens to the same plan.

use std::io::Read;

use bingrove_core::export::{ExportOptions, FileKind, PROJECT_FORMAT, ProjectFile, export};
use bingrove_core::model::{DrawerProject, DrawerSpec, PlateOptions};
use bingrove_core::plan::plan;
use bingrove_core::presets::Preset;

fn triangles_in_3mf(path: &std::path::Path) -> (usize, usize) {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut model = String::new();
    zip.by_name("3D/3dmodel.model").unwrap().read_to_string(&mut model).unwrap();
    (model.matches("<object ").count(), model.matches("<item ").count())
}

#[test]
fn drawer_export_writes_everything() {
    let project = DrawerProject {
        drawer: DrawerSpec { name: "Kleinkram 250×305".into(), width: 250.0, depth: 305.0, height: 100.0 },
        options: PlateOptions::default(),
        placements: Preset::get("250x305").unwrap().placements,
        ..DrawerProject::default()
    };
    let plan = plan(&project).unwrap();
    let out = std::env::temp_dir().join(format!("bingrove-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let opts = ExportOptions { assembly: true, ..ExportOptions::default() };
    let report = export(&plan, &project, &out, opts, &|_, _, _| {}).unwrap();

    let parts: Vec<_> = report.files.iter().filter(|f| f.kind == FileKind::Part).collect();
    assert_eq!(parts.len(), plan.parts.len());
    assert_eq!(parts.iter().map(|f| f.quantity).sum::<u32>(), plan.part_count());
    assert!(parts.iter().all(|f| out.join(&f.path).is_file()));

    let beds: Vec<_> = report.files.iter().filter(|f| f.kind == FileKind::Bed).collect();
    assert_eq!(beds.len(), report.beds);
    assert!(report.beds >= 3, "{}", report.beds);
    let items: usize = beds.iter().map(|f| triangles_in_3mf(&out.join(&f.path)).1).sum();
    assert_eq!(items as u32, plan.part_count());

    let assembly = report.files.iter().find(|f| f.kind == FileKind::Assembly).unwrap();
    let (objects, items) = triangles_in_3mf(&out.join(&assembly.path));
    assert_eq!(objects, plan.parts.len());
    assert_eq!(items, plan.pieces.len() + plan.placements.len());

    let pf = report.files.iter().find(|f| f.kind == FileKind::Project).unwrap();
    assert_eq!(pf.path, "kleinkram_250_305.gridfinity.json");
    let pf: ProjectFile = serde_json::from_str(&std::fs::read_to_string(out.join(&pf.path)).unwrap()).unwrap();
    assert_eq!(pf.format, PROJECT_FORMAT);
    let again = bingrove_core::plan::plan(&pf.project).unwrap();
    assert_eq!(again.parts, plan.parts);
    std::fs::remove_dir_all(&out).unwrap();
}
