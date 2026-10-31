//! Meshes for the panel's 3D view: the whole drawer, or one part.

use bingrove_core::geom;
use bingrove_core::mesh::TriMesh;
use bingrove_core::plan::Plan;
use bingrove_core::profile::Profile;
use meshgrove_plugin_api::MeshData;

/// Circle segments for previews: fast, still round enough to look right.
const PREVIEW_SEG: u32 = 24;

fn to_mesh_data(m: TriMesh) -> MeshData {
    MeshData { positions: m.positions.into_iter().flatten().collect(), indices: m.indices }
}

pub(crate) fn drawer(plan: &Plan, profile: &Profile) -> Option<MeshData> {
    let p = Profile { seg: PREVIEW_SEG, ..profile.clone() };
    geom::assembly_mesh(plan, &p).ok().map(to_mesh_data)
}

pub(crate) fn part(plan: &Plan, index: usize, profile: &Profile) -> Option<MeshData> {
    let p = Profile { seg: PREVIEW_SEG, ..profile.clone() };
    geom::part(plan, index, &p).mesh().ok().map(to_mesh_data)
}
