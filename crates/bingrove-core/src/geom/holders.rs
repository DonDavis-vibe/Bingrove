//! Holders: a bin shell with a solid insert and a pattern of pockets for one kind of thing.

use super::Part;
use super::bin::edge_bin;
use super::insert::with_pockets;
use super::kernel::{Solid, Vec3, cuboid, cylinder, hex_prism, polygon_prism, rrect_prism, subtract, translate, union};
use super::tape;
use crate::catalog::{
    HOLE_WALL, PATTERN_MARGIN, RING_WALL, ROW_LABEL_STRIP, across, bit_pitch, bits_grid, hole_pitch, holes_grid,
    ring_pitch, rings_grid, row_pitch, slot_count, trough_count,
};
use crate::model::{
    BIT_ACROSS_FLATS, BIT_CLEARANCE, BIT_DEPTH, BinSpec, CradleSpec, EdgeMode, HolderSpec, HoleShape, HolesSpec,
    RAMP_RISE, RingSpec, RowSpec, SlotSpec, TroughSpec, VSlotSpec,
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
    // Holes go down into the insert; pegs stand on a thin base above the floor.
    let reach = if spec.pegs.is_some() { PEG_BASE } else { spec.max_depth() };
    let base_top =
        (FLOOR_Z + FLOOR + reach).min(h_body - DIVIDER_GAP - step * (rows as f64 - 1.0)).max(FLOOR_Z + FLOOR + 1.0);
    let label = if spec.has_labels() { ROW_LABEL_STRIP } else { 0.0 };
    let total: f64 = spec.rows.iter().map(|r| row_pitch(spec, r)).sum();
    let inner_w = w - 2.0 * WALL + 0.02;
    let z0 = FLOOR_Z - 0.01;
    let mut y = d / 2.0 - total / 2.0;
    let mut blocks = vec![shell(spec.cells, units, p)];
    let mut pegs = Vec::new();
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
            match spec.pegs {
                Some(drive) => {
                    // A square peg standing on the strip, chamfered at the top.
                    let side = (drive.square() - p.peg_undersize).max(3.0);
                    pegs.push(translate(&peg(side, PEG_H), [x + a / 2.0, cy, top_k]));
                }
                None => {
                    let depth = h.depth().min(top_k - FLOOR_Z - FLOOR);
                    let hole = hole_solid(spec.shape, h.d + spec.clearance, depth, 0.8, p.seg);
                    // A tilted hole leans back about its mouth, so the mouth stays where it is.
                    let hole = translate(&hole, [0.0, 0.0, -depth]).rotate(-spec.tilt.clamp(0.0, 30.0), 0.0, 0.0);
                    cuts.push(translate(&hole, [x + a / 2.0, cy, top_k]));
                }
            }
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
    let body = subtract(union(blocks), cuts);
    if pegs.is_empty() { body } else { union(std::iter::once(body).chain(pegs).collect()) }
}

/// A square peg of `side` standing on z = 0, `height` tall, its top edges chamfered by 0.8 mm,
/// sunk 1 mm into whatever it stands on.
fn peg(side: f64, height: f64) -> Solid {
    let (h, c) = (side / 2.0, (side / 2.0 - 0.8).max(1.0));
    let corners = |r: f64, z: f64| [[-r, -r], [r, -r], [r, r], [-r, r]].map(|[x, y]| Vec3::new(x, y, z));
    let mut pts: Vec<Vec3> = corners(h, -1.0).to_vec();
    pts.extend(corners(h, height - 0.8));
    pts.extend(corners(c, height));
    Solid::hull(&pts)
}

/// Sloped beds side by side along x for things lying front to back: each place is a strip of
/// the insert whose top rises towards the back by the holder's angle, with the thing's bed cut
/// in parallel to it; every place further right stands a unit higher when stepped.
fn cradles(spec: &CradleSpec, units: u32, p: &Profile) -> Solid {
    let [w, d] = spec.cells.map(bin_outer);
    let h_body = body_height(units);
    let step = if spec.stepped { UNIT } else { 0.0 };
    let total: f64 = spec.slots.iter().map(|s| s.width + spec.clearance + HOLE_WALL).sum::<f64>() + HOLE_WALL;
    let (y0, y1) = (WALL - 0.01, d - WALL + 0.01);
    let inner_d = y1 - y0;
    let z0 = FLOOR_Z - 0.01;
    let mut x = w / 2.0 - total / 2.0;
    let mut blocks = vec![shell(spec.cells, units, p)];
    let mut cuts = Vec::new();
    for (k, slot) in spec.slots.iter().enumerate() {
        let width = slot.width + spec.clearance;
        let bed = CradleSpec::bed_depth(slot);
        // The strip: a wedge from the front (low) to the back (high), clipped to the body.
        let front_top = (FLOOR_Z + FLOOR + bed + 1.0 + k as f64 * step).min(h_body - DIVIDER_GAP);
        let back_top = (front_top + spec.rise_over(inner_d)).min(h_body - DIVIDER_GAP);
        let (xa, xb) = (
            if k == 0 { WALL - 0.01 } else { x },
            if k + 1 == spec.slots.len() { w - WALL + 0.01 } else { x + width + HOLE_WALL },
        );
        let wedge = Solid::hull(&[
            Vec3::new(xa, y0, z0),
            Vec3::new(xb, y0, z0),
            Vec3::new(xb, y1, z0),
            Vec3::new(xa, y1, z0),
            Vec3::new(xa, y0, front_top),
            Vec3::new(xb, y0, front_top),
            Vec3::new(xb, y1, back_top),
            Vec3::new(xa, y1, back_top),
        ]);
        blocks.push(wedge);
        // The bed: a box as long as the thing, lying parallel to the wedge's top, `bed` deep.
        let length = slot.length.min(inner_d - 1.0);
        let slope = (back_top - front_top) / inner_d;
        let tilt = slope.atan().to_degrees();
        let box_ = cuboid([width, length + 1.0, bed + 50.0], [0.0, -1.0, -bed]).rotate(tilt, 0.0, 0.0);
        cuts.push(translate(&box_, [x + HOLE_WALL, y0 + 1.0, front_top + slope * 1.0]));
        x += width + HOLE_WALL;
    }
    subtract(union(blocks), cuts)
}

/// V-bottomed slots side by side along x for things standing on edge, open to the front (and
/// the back unless closed), the bottoms rising towards the back on a ramp.
fn vslots(spec: &VSlotSpec, units: u32, p: &Profile) -> Solid {
    let [w, d] = spec.cells.map(bin_outer);
    let h_body = body_height(units);
    let top = (FLOOR_Z + FLOOR + spec.max_depth()).min(h_body - DIVIDER_GAP);
    let total: f64 = spec.slots.iter().map(|s| s.width + spec.clearance + HOLE_WALL).sum::<f64>() + HOLE_WALL;
    let (y0, y1) = (-1.0, if spec.closed_end { d - WALL - HOLE_WALL } else { d + 1.0 });
    let rise = if spec.ramp { RAMP_RISE } else { 0.0 };
    let mut x = w / 2.0 - total / 2.0 + HOLE_WALL;
    let mut cuts = Vec::new();
    for slot in &spec.slots {
        let width = slot.width + spec.clearance;
        // `depth` reaches the tip of the V; on a ramp the front end goes `rise` deeper, and
        // nothing goes into the floor.
        let depth = slot.depth.min(top - FLOOR_Z - FLOOR - rise).max(1.0);
        let v = width / 2.0;
        let profile: Vec<[f64; 2]> = vec![[0.0, -depth], [v, -depth + v], [v, 50.0], [-v, 50.0], [-v, -depth + v]];
        // The prism lies along y from -(y1 - y0) to 0 after the turn; tilting it about x lowers
        // its front end, so the bottoms rise towards the back.
        let prism = polygon_prism(&profile, 0.0, y1 - y0, 8).rotate(90.0, 0.0, 0.0);
        let tilt = (rise / (y1 - y0)).atan().to_degrees();
        let prism = prism.rotate(tilt, 0.0, 0.0);
        cuts.push(translate(&prism, [x + v, y1, top]));
        x += width + HOLE_WALL;
    }
    with_pockets(shell(spec.cells, units, p), w, d, top, cuts, p.seg)
}

/// Rings standing on the bin floor, as many as fit (or as asked), for round things to sit in.
fn rings(spec: &RingSpec, units: u32, p: &Profile) -> Solid {
    let [w, d] = spec.cells.map(bin_outer);
    let h_body = body_height(units);
    let height = spec.height.min(h_body - DIVIDER_GAP - FLOOR_Z);
    let r_in = (spec.d + spec.clearance) / 2.0;
    let ring = cylinder(height + 0.5, r_in + RING_WALL, r_in + RING_WALL, p.seg)
        - translate(&cylinder(height + 2.0, r_in, r_in, p.seg), [0.0, 0.0, 1.0]);
    let parts = centres(w, d, rings_grid(spec), ring_pitch(spec))
        .into_iter()
        .map(|(x, y)| translate(&ring, [x, y, FLOOR_Z - 0.5]))
        .chain(std::iter::once(shell(spec.cells, units, p)))
        .collect();
    union(parts)
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
        HolderSpec::Cradles(c) => c.title.as_deref(),
        HolderSpec::VSlots(v) => v.title.as_deref(),
        HolderSpec::Rings(r) => r.label.as_deref(),
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
        HolderSpec::Cradles(c) => cradles(c, units, p),
        HolderSpec::VSlots(v) => vslots(v, units, p),
        HolderSpec::Rings(r) => rings(r, units, p),
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
