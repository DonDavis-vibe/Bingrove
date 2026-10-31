//! Holders: a bin shell with a solid insert and a pattern of pockets for one kind of thing.

use super::Part;
use super::bin::edge_bin;
use super::insert::{block, with_pockets};
use super::kernel::{Solid, cylinder, hex_prism, translate, union};
use super::tape;
use crate::catalog::{HOLE_WALL, bit_pitch, bits_grid, hole_pitch, holes_grid, trough_count};
use crate::model::{BIT_ACROSS_FLATS, BIT_CLEARANCE, BIT_DEPTH, BinSpec, EdgeMode, HolderSpec, HolesSpec, TroughSpec};
use crate::profile::Profile;
use crate::spec::*;

/// Centres of an `n` × `m` pattern with `pitch`, centred in a `w` × `d` bin.
fn centres(w: f64, d: f64, n: [u32; 2], pitch: f64) -> Vec<(f64, f64)> {
    let x0 = w / 2.0 - (n[0] as f64 - 1.0) * pitch / 2.0;
    let y0 = d / 2.0 - (n[1] as f64 - 1.0) * pitch / 2.0;
    (0..n[0]).flat_map(|i| (0..n[1]).map(move |j| (x0 + i as f64 * pitch, y0 + j as f64 * pitch))).collect()
}

/// The plain bin a holder is built from: no dividers, no cutouts, the holder's height.
fn shell(cells: [u32; 2], units: u32, p: &Profile) -> Solid {
    let spec = BinSpec { cells, units, ..BinSpec::default() };
    edge_bin(&spec, [0.0; 4], EdgeMode::default(), 1.0, p).solid
}

fn holes(spec: &HolesSpec, units: u32, p: &Profile) -> Solid {
    let (dia, h) = spec.item.size();
    let [w, d] = spec.cells.map(bin_outer);
    let h_body = body_height(units);
    let top = (FLOOR_Z + h * 0.55 + FLOOR).min(h_body - DIVIDER_GAP);
    let depth = top - FLOOR_Z - FLOOR;
    let r = (dia + spec.clearance) / 2.0;
    let hole = cylinder(depth + 1.0, r, r, p.seg)
        + translate(&cylinder(0.8, r, r + 0.8, p.seg), [0.0, 0.0, depth - 0.8 + 1.0]);
    let cuts = centres(w, d, holes_grid(spec), hole_pitch(&spec.item, spec.clearance))
        .into_iter()
        .map(|(x, y)| translate(&hole, [x, y, top - depth]))
        .collect();
    with_pockets(shell(spec.cells, units, p), w, d, top, cuts, p.seg)
}

fn bits(cells: [u32; 2], units: u32, p: &Profile) -> Solid {
    let [w, d] = cells.map(bin_outer);
    let h_body = body_height(units);
    let top = (FLOOR_Z + BIT_DEPTH + FLOOR).min(h_body - DIVIDER_GAP);
    let depth = top - FLOOR_Z - FLOOR;
    let hole = hex_prism(BIT_ACROSS_FLATS + BIT_CLEARANCE, depth + 1.0);
    let cuts = centres(w, d, bits_grid(cells), bit_pitch())
        .into_iter()
        .map(|(x, y)| translate(&hole, [x, y, top - depth]))
        .collect();
    with_pockets(shell(cells, units, p), w, d, top, cuts, p.seg)
}

fn troughs(spec: &TroughSpec, units: u32, p: &Profile) -> Solid {
    let [w, d] = spec.cells.map(bin_outer);
    let h_body = body_height(units);
    let r = spec.diameter / 2.0 + 0.5;
    let rh = spec.handle.map(|h| h / 2.0 + 0.5);
    let top = (FLOOR_Z + FLOOR + rh.unwrap_or(r) * 1.2).min(h_body - DIVIDER_GAP);
    let n = trough_count(spec);
    let pitch = spec.handle.unwrap_or(spec.diameter) + 1.0 + HOLE_WALL;
    let y0 = d / 2.0 - (n as f64 - 1.0) * pitch / 2.0;
    // Lying along x, axis a little below the insert's top so the thing sits in, not on it.
    let axis = (top - r * 0.4).max(FLOOR_Z + FLOOR + r * 0.6);
    let shaft = cylinder(w + 2.0, r, r, p.seg).rotate(0.0, 90.0, 0.0);
    let mut cuts = Vec::new();
    for k in 0..n {
        let y = y0 + k as f64 * pitch;
        cuts.push(translate(&shaft, [-1.0, y, axis]));
        if let Some(rh) = rh {
            // The handle lies on the back 45 % of the holder.
            let len = w * 0.45 + 1.0;
            let handle = cylinder(len, rh, rh, p.seg).rotate(0.0, 90.0, 0.0);
            cuts.push(translate(&handle, [w - len + 1.0, y, (axis + rh - r).max(axis)]));
        }
    }
    // Everything above the axis opens up so the things can be lifted out.
    let opening = cuts.iter().map(|c| translate(c, [0.0, 0.0, (top - axis).max(0.0) + 0.01])).collect::<Vec<_>>();
    cuts.extend(opening);
    let _ = block; // the insert comes with `with_pockets`
    with_pockets(shell(spec.cells, units, p), w, d, top, cuts, p.seg)
}

pub fn holder(h: &HolderSpec, p: &Profile) -> Part {
    let solid = match h {
        HolderSpec::Tape(t) => return tape::tape(t, p),
        HolderSpec::Holes(s) => holes(s, h.units(), p),
        HolderSpec::Bits(b) => bits(b.cells, h.units(), p),
        HolderSpec::Troughs(t) => troughs(t, h.units(), p),
    };
    Part::new(crate::names::holder(h), union(vec![solid]))
}
