//! A solid insert inside a bin with pockets cut into it: what holders and cutouts share.
//! Everything in the bin's final coordinates (minimum corner at the origin).

use super::kernel::{Solid, cylinder, polygon_prism, rrect_prism, subtract, translate, union};
use crate::model::Cutout;
use crate::spec::{BIN_R, FLOOR_Z, WALL};

/// The insert block from the bin floor up to `top`, inside the walls of a bin of outer size `w` × `d`.
pub fn block(w: f64, d: f64, top: f64, seg: u32) -> Solid {
    let z0 = FLOOR_Z - 0.01;
    translate(
        &rrect_prism(w - 2.0 * WALL + 0.02, d - 2.0 * WALL + 0.02, BIN_R - WALL, top - z0, seg),
        [w / 2.0, d / 2.0, z0],
    )
}

/// `shell` with the insert added and `cuts` taken out of it from `top` downwards.
pub fn with_pockets(shell: Solid, w: f64, d: f64, top: f64, cuts: Vec<Solid>, seg: u32) -> Solid {
    let body = union(vec![shell, block(w, d, top, seg)]);
    subtract(body, cuts)
}

/// The solid of one cutout, cut from `top` down by its depth, positioned from the bin's front-left corner.
pub fn cutout_solid(c: &Cutout, top: f64, seg: u32) -> Solid {
    let big = 1.0;
    match c {
        Cutout::Box { at, size, depth } => {
            let r = (size[0].min(size[1]) / 2.0).min(2.0);
            translate(&rrect_prism(size[0], size[1], r, depth + big, seg), [at[0], at[1], top - depth])
        }
        Cutout::Cylinder { at, d, depth } => {
            translate(&cylinder(depth + big, d / 2.0, d / 2.0, seg), [at[0], at[1], top - depth])
        }
        Cutout::Polygon { points, clearance, depth } => {
            translate(&polygon_prism(points, *clearance, depth + big, seg), [0.0, 0.0, top - depth])
        }
    }
}

/// Where the insert's top goes for these cutouts: the deepest one plus a floor, but never above `max`.
pub fn insert_top(cutouts: &[Cutout], max: f64) -> f64 {
    let deepest = cutouts.iter().map(Cutout::depth).fold(0.0, f64::max);
    (FLOOR_Z + deepest + crate::spec::FLOOR).min(max)
}
