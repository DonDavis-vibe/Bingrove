//! Solids of the parts. Every part is made with its minimum corner at the origin, ready to print.

pub mod baseplate;
pub mod bin;
pub mod kernel;
pub mod tape;

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
            baseplate::pieces_of(&plan.grid, &plan.claims, pieces, plan.magnets, p).pop().expect("one piece")
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
        PartSpec::Tape(t) => tape::tape(t, p),
    }
}

/// Every part of the plan, the plate computed once for all its pieces.
pub fn all_parts(plan: &Plan, p: &Profile) -> Vec<Part> {
    let mut plates = baseplate::pieces_of(&plan.grid, &plan.claims, &plan.pieces, plan.magnets, p).into_iter();
    (0..plan.parts.len())
        .map(|i| match plan.parts[i].part {
            PartSpec::Plate { .. } => plates.next().expect("one solid per piece"),
            _ => part(plan, i, p),
        })
        .collect()
}
