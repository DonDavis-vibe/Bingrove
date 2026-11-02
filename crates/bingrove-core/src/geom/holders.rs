//! Holders: a bin shell with a solid insert and a pattern of pockets for one kind of thing.

use super::Part;
use super::bin::edge_bin;
use super::insert::with_pockets;
use super::kernel::{Solid, cuboid, cylinder, hex_prism, rrect_prism, subtract, translate, union};
use super::tape;
use crate::catalog::{
    HOLE_WALL, PATTERN_MARGIN, ROW_LABEL_STRIP, across, bit_pitch, bits_grid, hole_pitch, holes_grid, row_pitch,
    slot_count, trough_count,
};
use crate::model::{
    BIT_ACROSS_FLATS, BIT_CLEARANCE, BIT_DEPTH, BinSpec, EdgeMode, HolderSpec, HoleShape, HolesSpec, RowSpec, SlotSpec,
    TroughSpec,
};
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

/// One hole to cut: `size` across (flats for hex, the side for square) with its mouth at
/// z = `depth`, a 45° chamfer of `chamfer` there, and 1 mm more above for a clean cut.
fn hole_solid(shape: HoleShape, size: f64, depth: f64, chamfer: f64, seg: u32) -> Solid {
    let body = match shape {
        HoleShape::Round => cylinder(depth + 1.0, size / 2.0, size / 2.0, seg),
        HoleShape::Hex => hex_prism(size, depth + 1.0),
        HoleShape::Square => rrect_prism(size, size, 1.0, depth + 1.0, seg),
    };
    if chamfer <= 0.0 {
        return body;
    }
    // The chamfer is round whatever the hole: a cone from the hole's widest reach outwards.
    let r = across(shape, size) / 2.0;
    let cone = cylinder(chamfer, r, r + chamfer, seg)
        + translate(&cylinder(1.0, r + chamfer, r + chamfer, seg), [0.0, 0.0, chamfer]);
    body + translate(&cone, [0.0, 0.0, depth - chamfer])
}

fn holes(spec: &HolesSpec, units: u32, p: &Profile) -> Solid {
    let [w, d] = spec.cells.map(bin_outer);
    let h_body = body_height(units);
    let top = (FLOOR_Z + spec.depth() + FLOOR).min(h_body - DIVIDER_GAP);
    let depth = top - FLOOR_Z - FLOOR;
    let hole = hole_solid(spec.shape, spec.item.size().0 + spec.clearance, depth, spec.chamfer, p.seg);
    let cuts = centres(w, d, holes_grid(spec), hole_pitch(spec))
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

/// Rows of holes with their own sizes, the front row first. The insert is one strip per row,
/// each a step higher than the one before when `stepped`; a label strip lies in front of every
/// row of holes when the holder carries text (the engraving itself comes with the text module).
fn row(spec: &RowSpec, units: u32, p: &Profile) -> Solid {
    let [w, d] = spec.cells.map(bin_outer);
    let h_body = body_height(units);
    let rows = spec.rows.len().max(1);
    let step = if spec.stepped { UNIT } else { 0.0 };
    let base_top = (FLOOR_Z + FLOOR + spec.max_depth())
        .min(h_body - DIVIDER_GAP - step * (rows as f64 - 1.0))
        .max(FLOOR_Z + FLOOR + 1.0);
    let label = if spec.has_labels() { ROW_LABEL_STRIP } else { 0.0 };
    let total: f64 = spec.rows.iter().map(|r| row_pitch(spec, r)).sum();
    let inner_w = w - 2.0 * WALL + 0.02;
    let z0 = FLOOR_Z - 0.01;
    let mut y = d / 2.0 - total / 2.0;
    let mut blocks = vec![shell(spec.cells, units, p)];
    let mut cuts = Vec::new();
    for (k, holes) in spec.rows.iter().enumerate() {
        let pitch = row_pitch(spec, holes);
        let top_k = (base_top + k as f64 * step).min(h_body - DIVIDER_GAP);
        let widest = holes.iter().map(|h| across(spec.shape, h.d + spec.clearance)).fold(0.0, f64::max);
        let cy = y + label + HOLE_WALL / 2.0 + widest / 2.0;
        let row_w: f64 =
            holes.iter().map(|h| across(spec.shape, h.d + spec.clearance) + HOLE_WALL).sum::<f64>() + HOLE_WALL;
        let mut x = w / 2.0 - row_w / 2.0 + HOLE_WALL;
        for h in holes {
            let a = across(spec.shape, h.d + spec.clearance);
            let depth = h.depth().min(top_k - FLOOR_Z - FLOOR);
            let hole = hole_solid(spec.shape, h.d + spec.clearance, depth, 0.8, p.seg);
            cuts.push(translate(&hole, [x + a / 2.0, cy, top_k - depth]));
            // The size in front of its hole, as large as the strip and the hole's width allow.
            let text = h.label.trim();
            if label > 0.0 && !text.is_empty() {
                let height = super::text::fit_height(text, ROW_LABEL_STRIP - 2.0, a + HOLE_WALL - 0.6).max(1.5);
                let body = super::text::text_solid(text, height, super::text::stroke_for(height), ENGRAVE + 0.5);
                let tx = x + a / 2.0 - super::text::text_width(text, height) / 2.0;
                cuts.push(translate(&body, [tx, y + (label - height) / 2.0, top_k - ENGRAVE]));
            }
            x += a + HOLE_WALL;
        }
        // The strip from the front wall (first row) or this row's start to its end or the back wall.
        let y0 = if k == 0 { WALL - 0.01 } else { y };
        let y1 = if k == rows - 1 { d - WALL + 0.01 } else { y + pitch };
        blocks.push(cuboid([inner_w, y1 - y0, top_k - z0], [WALL - 0.01, y0, z0]));
        y += pitch;
    }
    subtract(union(blocks), cuts)
}

/// Parallel slots along y, spaced along x, for things standing on edge.
fn slots(spec: &SlotSpec, units: u32, p: &Profile) -> Solid {
    let [w, d] = spec.cells.map(bin_outer);
    let h_body = body_height(units);
    let top = (FLOOR_Z + FLOOR + spec.depth).min(h_body - DIVIDER_GAP);
    let depth = top - FLOOR_Z - FLOOR;
    let n = slot_count(spec);
    let room = d - 2.0 * WALL - 2.0 * PATTERN_MARGIN;
    let length = spec.length.unwrap_or(room).min(room).max(spec.width);
    let slot = rrect_prism(spec.width, length, (spec.width / 2.0).min(1.5), depth + 1.0, p.seg);
    let x0 = w / 2.0 - (n as f64 - 1.0) * spec.pitch / 2.0;
    let cuts = (0..n).map(|i| translate(&slot, [x0 + i as f64 * spec.pitch, d / 2.0, top - depth])).collect();
    with_pockets(shell(spec.cells, units, p), w, d, top, cuts, p.seg)
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
    with_pockets(shell(spec.cells, units, p), w, d, top, cuts, p.seg)
}

/// How deep text goes into a face.
const ENGRAVE: f64 = 0.6;

/// The text a holder shows on its front wall.
fn front_text(h: &HolderSpec) -> Option<&str> {
    match h {
        HolderSpec::Holes(s) => s.label.as_deref(),
        HolderSpec::Slots(s) => s.label.as_deref(),
        HolderSpec::Row(r) => r.title.as_deref(),
        _ => None,
    }
    .map(str::trim)
    .filter(|t| !t.is_empty())
}

pub fn holder(h: &HolderSpec, p: &Profile) -> Part {
    let units = h.units();
    let mut solid = match h {
        HolderSpec::Tape(t) => return tape::tape(t, p),
        HolderSpec::Holes(s) => holes(s, units, p),
        HolderSpec::Bits(b) => bits(b.cells, units, p),
        HolderSpec::Troughs(t) => troughs(t, units, p),
        HolderSpec::Row(r) => row(r, units, p),
        HolderSpec::Slots(s) => slots(s, units, p),
    };
    if let Some(text) = front_text(h) {
        let w = bin_outer(h.cells()[0]);
        let h_body = body_height(units);
        let height = super::text::fit_height(text, (h_body - FLOOR_Z - 3.0).min(6.0), w - 2.0 * WALL - 4.0);
        if height >= 2.0 {
            let z = FLOOR_Z + (h_body - FLOOR_Z) / 2.0;
            solid = super::text::engrave_front(solid, text, [w / 2.0, 0.0, z], height, 0.5);
        }
    }
    Part::new(crate::names::holder(h), union(vec![solid]))
}
