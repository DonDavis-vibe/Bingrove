//! Solids of the parts. Every part is made with its minimum corner at the origin, ready to print.

pub mod baseplate;
pub mod bin;
pub mod calibration;
pub mod holders;
pub mod insert;
pub mod kernel;
pub mod lid;
pub mod tape;
pub mod text;

use kernel::{GeomError, Solid};

use crate::mesh::TriMesh;
use crate::plan::{PartSpec, Plan};
use crate::profile::Profile;

/// A finished part: its file name and its solid, minimum corner at the origin.
pub struct Part {
    pub name: String,
    pub solid: Solid,
    /// Where the minimum corner belongs in plate coordinates (plate pieces; bins are placed
    /// through [`Plan::position`]).
    pub origin: [f64; 2],
}

impl Part {
    pub fn new(name: String, solid: Solid) -> Self {
        Part { name, solid, origin: [0.0; 2] }
    }

    pub fn mesh(&self) -> Result<TriMesh, GeomError> {
        kernel::finish(&self.solid, &self.name)
    }
}

/// The solid of one entry of the plan's parts list.
pub fn part(plan: &Plan, index: usize, p: &Profile) -> Part {
    match &plan.parts[index].part {
        PartSpec::Plate { piece } => {
            let pieces = std::slice::from_ref(&plan.pieces[*piece]);
            baseplate::pieces_of(&plan.grid, &plan.claims, pieces, plan.floor, plan.mount, p).pop().expect("one piece")
        }
        PartSpec::Bin { spec, extents, edge_mode } => {
            let mut spec = spec.clone();
            if plan.magnets && spec.magnets == crate::model::MagnetMode::None {
                spec.magnets = crate::model::MagnetMode::Corners;
            }
            let mut part = bin::edge_bin(&spec, *extents, *edge_mode, plan.grid.rim, p);
            part.name = plan.parts[index].name.clone();
            part
        }
        PartSpec::Holder(h) => holders::holder(h, p),
        PartSpec::Lid(l) => lid::lid(l, p),
    }
}

/// The whole drawer as one triangle mesh for a preview: every piece and every bin in place.
/// No booleans, the parts just sit next to each other.
pub fn assembly_mesh(plan: &Plan, p: &Profile) -> Result<TriMesh, GeomError> {
    let parts = all_parts(plan, p);
    let meshes: Vec<TriMesh> = parts.iter().map(Part::mesh).collect::<Result<_, _>>()?;
    Ok(assemble(plan, &parts, &meshes))
}

/// The assembly from meshes already made: `meshes[i]` is the mesh of `parts[i]`, the plan's
/// parts in order.
pub fn assemble(plan: &Plan, parts: &[Part], meshes: &[TriMesh]) -> TriMesh {
    let mut out = TriMesh::default();
    for (i, part) in parts.iter().enumerate() {
        if matches!(plan.parts[i].part, PartSpec::Plate { .. }) {
            out.append(&meshes[i].clone().translated([part.origin[0] as f32, part.origin[1] as f32, 0.0]));
        }
    }
    for placed in &plan.placements {
        let mut m = meshes[placed.part].clone();
        let [x, y, z] = plan.position(placed);
        if placed.turned {
            let size = plan.parts[placed.part].size;
            m = m.turned_180([size[0] as f32, size[1] as f32]);
        }
        out.append(&m.translated([x as f32, y as f32, z as f32]));
    }
    out
}

/// Every part of the plan, the plate computed once for all its pieces.
pub fn all_parts(plan: &Plan, p: &Profile) -> Vec<Part> {
    let mut plates =
        baseplate::pieces_of(&plan.grid, &plan.claims, &plan.pieces, plan.floor, plan.mount, p).into_iter();
    (0..plan.parts.len())
        .map(|i| match plan.parts[i].part {
            PartSpec::Plate { .. } => plates.next().expect("one solid per piece"),
            _ => part(plan, i, p),
        })
        .collect()
}
