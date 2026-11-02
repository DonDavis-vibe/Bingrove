//! Meshes for the panel's 3D view. A whole drawer takes seconds in the sandbox and a panel call
//! has two, so a job computes them (no deadline, with progress) and leaves them in
//! `/data/preview`; the panel only reads files. One file per part and one for the assembly,
//! named by a key over everything the geometry depends on.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use bingrove_core::geom;
use bingrove_core::mesh::TriMesh;
use bingrove_core::model::DrawerProject;
use bingrove_core::plan::{PartSpec, Plan, plan};
use bingrove_core::profile::Profile;
use meshgrove_plugin_api::MeshData;
use meshgrove_plugin_api::guest::{self, PluginError};

/// Circle segments for previews: fast, still round enough to look right.
const PREVIEW_SEG: u32 = 24;
const DIR: &str = "/data/preview";
/// Previews kept, so coming back to an earlier state needs no new job.
const KEEP: usize = 6;

/// The key of a plan's preview: a hash over the plan and the profile.
pub(crate) fn key(plan: &Plan, profile: &Profile) -> String {
    let mut h = DefaultHasher::new();
    serde_json::to_string(plan).unwrap_or_default().hash(&mut h);
    serde_json::to_string(profile).unwrap_or_default().hash(&mut h);
    PREVIEW_SEG.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn file(key: &str, part: Option<usize>) -> PathBuf {
    match part {
        Some(i) => format!("{DIR}/{key}-{i}.bin").into(),
        None => format!("{DIR}/{key}.bin").into(),
    }
}

/// Whether the preview with `key` is there to read.
pub(crate) fn ready(key: &str) -> bool {
    file(key, None).exists()
}

/// The job: every part's mesh and the assembly, written for the panel. Returns the part count.
pub(crate) fn compute(project: &DrawerProject, key: &str) -> Result<usize, PluginError> {
    let plan = plan(project).map_err(|e| PluginError::Invalid(e.to_string()))?;
    let profile = Profile { seg: PREVIEW_SEG, ..project.profile.clone() };
    std::fs::create_dir_all(DIR)?;
    let n = plan.parts.len();
    // The plate once for all its pieces, the bins one by one so the progress bar moves.
    guest::host::progress(0.0, "Baseplate");
    let mut plates =
        geom::baseplate::pieces_of(&plan.grid, &plan.claims, &plan.pieces, plan.floor, plan.mount, &profile)
            .into_iter();
    let mut parts = Vec::with_capacity(n);
    let mut meshes = Vec::with_capacity(n);
    for i in 0..n {
        if guest::host::cancelled() {
            return Err(PluginError::Cancelled);
        }
        let part = match plan.parts[i].part {
            PartSpec::Plate { .. } => {
                plates.next().ok_or_else(|| PluginError::Failed("a plate piece is missing".into()))?
            }
            _ => {
                guest::host::progress(i as f32 / (n + 1) as f32, &plan.parts[i].name);
                geom::part(&plan, i, &profile)
            }
        };
        let mesh = part.mesh().map_err(|e| PluginError::Failed(e.to_string()))?;
        write(&file(key, Some(i)), &mesh)?;
        parts.push(part);
        meshes.push(mesh);
    }
    guest::host::progress(n as f32 / (n + 1) as f32, "Zusammenbau");
    write(&file(key, None), &geom::assemble(&plan, &parts, &meshes))?;
    prune(key);
    Ok(n)
}

/// Positions and indices, little-endian, behind their counts.
fn write(path: &Path, m: &TriMesh) -> std::io::Result<()> {
    let mut bytes = Vec::with_capacity(8 + m.positions.len() * 12 + m.indices.len() * 4);
    bytes.extend((m.positions.len() as u32).to_le_bytes());
    bytes.extend((m.indices.len() as u32).to_le_bytes());
    bytes.extend(m.positions.iter().flatten().flat_map(|v| v.to_le_bytes()));
    bytes.extend(m.indices.iter().flat_map(|i| i.to_le_bytes()));
    std::fs::write(path, bytes)
}

/// The assembly (`None`) or part `part` of the preview with `key`; `None` when it is not there.
pub(crate) fn read(key: &str, part: Option<usize>) -> Option<MeshData> {
    let bytes = std::fs::read(file(key, part)).ok()?;
    let (head, rest) = bytes.split_at_checked(8)?;
    let npos = u32::from_le_bytes(head[..4].try_into().ok()?) as usize;
    let nidx = u32::from_le_bytes(head[4..].try_into().ok()?) as usize;
    if rest.len() != npos * 12 + nidx * 4 {
        return None;
    }
    let (pos, idx) = rest.split_at(npos * 12);
    let positions = pos.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
    let indices = idx.as_chunks::<4>().0.iter().map(|b| u32::from_le_bytes(*b)).collect();
    Some(MeshData { positions, indices })
}

/// Keeps the `KEEP` newest previews (and `keep`), deletes the rest.
fn prune(keep: &str) {
    let Ok(dir) = std::fs::read_dir(DIR) else { return };
    let mut newest: HashMap<String, std::time::SystemTime> = HashMap::new();
    let mut files = Vec::new();
    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".bin") else { continue };
        let k = stem.split_once('-').map_or(stem, |(k, _)| k).to_owned();
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else { return };
        let t = newest.entry(k.clone()).or_insert(modified);
        *t = (*t).max(modified);
        files.push((k, entry.path()));
    }
    let mut keys: Vec<_> = newest.into_iter().collect();
    keys.sort_by_key(|k| std::cmp::Reverse(k.1));
    let old: Vec<String> = keys.into_iter().skip(KEEP).map(|(k, _)| k).filter(|k| k != keep).collect();
    for (k, path) in files {
        if old.contains(&k) {
            let _ = std::fs::remove_file(path);
        }
    }
}
