//! Wall parts: bins and holders that hang on a wall plate.
//!
//! A wall plate is a surface turned up: x across, y up the wall, z out of it. Every cell's
//! pocket has a hook standing on its floor. A wall part is `HOOK_LIFT` shorter at the top than
//! a bin, so it goes straight into the pockets lifted by that much and then slides down; the
//! hook runs into a keyhole in each foot, and the foot material left under the hook's finger
//! keeps the part from being pulled straight out. Lifting it frees it again.
//!
//! Wall parts are made in their print frame: lying on their bottom face (z up = up the wall),
//! the front at y = 0, the feet at the back reaching to y = depth, their pockets opening
//! upwards. Seen from the plate, print (x, y, z) is plate (x, z, depth − y), so standing the
//! assembly up makes a wall part a plain shift.

use super::Part;
use super::bin::magnet_hole;
use super::holders::hole_solid;
use super::kernel::{Solid, cuboid, cylinder, polygon_prism, rrect_prism, subtract, swept, translate, union};
use super::text;
use crate::catalog::HOLE_WALL;
use crate::model::{BinSpec, HolderSpec, MagnetMode, WallCradleSpec, WallHolesSpec, WallVSlotSpec};
use crate::profile::Profile;
use crate::spec::*;

/// Up the wall from a cell's lower grid line, where a wall plate's and a wall part's magnets
/// sit: 12 mm either side of the middle of the foot, which is `HOOK_LIFT` shorter at the top.
pub const WALL_MAGNET_Y: [f64; 2] = [FOOT_MID - 12.0, FOOT_MID + 12.0];
const FOOT_MID: f64 = BIN_GAP + (BIN_SIZE - HOOK_LIFT) / 2.0;

pub use crate::spec::wall_size as size;

/// The hook's side view (y up the wall, z out of it) as convex pieces: the stem, the gusset
/// with the root of the finger, and the rest of the finger with its tip chamfered underneath.
fn hook_pieces() -> [Vec<[f64; 2]>; 3] {
    let (y0, s, r, h) = (HOOK_Y0, HOOK_STEM, HOOK_REACH, HOOK_H);
    let (under, g, tip) = (HOOK_H - HOOK_FINGER, HOOK_GUSSET, 0.5);
    [
        vec![[y0, -0.01], [y0 + s, -0.01], [y0 + s, h], [y0, h]],
        vec![[y0 + s, under - g], [y0 + s + g, under], [y0 + s + g, h], [y0 + s, h]],
        vec![[y0 + s + g, under], [y0 + r - tip, under], [y0 + r, under + tip], [y0 + r, h], [y0 + s + g, h]],
    ]
}

/// A side view (y, z), counter-clockwise, made a solid across x, `half` either side of the
/// cell centre.
fn across(pts: &[[f64; 2]], half: f64) -> Solid {
    // Extruded along z from the (y, z) outline drawn in the xy-plane, then (a, b, c) → (c, a, b).
    translate(&polygon_prism(pts, 0.0, 2.0 * half, 8).rotate(90.0, 0.0, 90.0), [-half, 0.0, 0.0])
}

/// The hook in a cell's frame on the plate: x from the cell centre, y up the wall from the
/// cell's lower grid line, z out of the wall from the pocket floor.
pub fn hook() -> Solid {
    union(hook_pieces().iter().map(|piece| across(piece, HOOK_W / 2.0)).collect())
}

/// The convex hull of points in a plane (counter-clockwise).
fn hull2d(mut pts: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    pts.dedup();
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut out: Vec<[f64; 2]> = Vec::new();
    for pass in 0..2 {
        let start = out.len();
        let iter: Box<dyn Iterator<Item = &[f64; 2]>> =
            if pass == 0 { Box::new(pts.iter()) } else { Box::new(pts.iter().rev()) };
        for &p in iter {
            while out.len() >= start + 2 && cross(out[out.len() - 2], out[out.len() - 1], p) <= 0.0 {
                out.pop();
            }
            out.push(p);
        }
        out.pop();
    }
    out
}

/// The room a hook needs in a foot, in the cell frame of a part seated on the plate, with
/// `gap` all round. The part goes in straight while lifted by `HOOK_LIFT` (up to
/// `HOOK_OVERTRAVEL` more), so the hook comes in from below through the channel its shadow cuts;
/// then the part slides down past its seat by the air under it (it settles onto its lower foot
/// faces), and each piece of the hook sweeps its way up the foot. What is left under the
/// finger is the tooth that keeps the part from being pulled straight out.
pub fn keyhole(gap: f64) -> Solid {
    let half = HOOK_W / 2.0 + gap;
    let (low, high) = (-(HOOK_LIFT + HOOK_OVERTRAVEL), BIN_GAP);
    let mut cuts: Vec<Solid> = hook_pieces()
        .iter()
        .enumerate()
        .map(|(i, piece)| {
            let mut pts = Vec::new();
            for &[y, z] in piece {
                // The stem stands on the floor: its way is open to the foot's underside.
                let z_low = if i == 0 && z < 0.0 { -1.0 } else { z - gap };
                for dy in [low - gap, high + gap] {
                    pts.push([y + dy, z_low]);
                    pts.push([y + dy, z + gap]);
                }
            }
            across(&hull2d(pts), half)
        })
        .collect();
    // The way in: lifted by at least `lift_in`, the whole hook passes below the tooth.
    let lift_in = HOOK_REACH - HOOK_STEM + 2.0 * gap;
    let (y0, y1) = (HOOK_Y0 + low - gap, HOOK_Y0 + HOOK_REACH - lift_in + gap);
    cuts.push(cuboid([2.0 * half, y1 - y0, HOOK_H + gap + 1.0], [-half, y0, -1.0]));
    union(cuts)
}

/// How far along the sliding direction a flank `depth` deep runs: 45°, or 60° when `steep`.
fn flank(depth: f64, steep: bool) -> f64 {
    if steep { depth / 3f64.sqrt() } else { depth }
}

/// Length of the blade's nose along the wall: a steep lower flank (it holds), the flat and a
/// 45° upper flank (the rib slides over it on the way down).
fn nose_len() -> f64 {
    flank(DETENT_NOSE, true) + DETENT_FLAT + flank(DETENT_NOSE, false)
}

/// The latch's spring blade in a cell's frame on the plate (like `hook`).
pub fn blade() -> Solid {
    let (x0, x1, n) = (-DETENT_T / 2.0, DETENT_T / 2.0, DETENT_NOSE);
    let (y0, y1) = (DETENT_Y0, DETENT_Y0 + DETENT_LEN);
    let ya = y1 - nose_len();
    let pts = [
        [x0, y0],
        [x1, y0],
        [x1, ya],
        [x1 + n, ya + flank(n, true)],
        [x1 + n, y1 - flank(n, false)],
        [x1, y1],
        [x0, y1],
    ];
    translate(&polygon_prism(&pts, 0.0, DETENT_H + 0.01, 8), [0.0, 0.0, -0.01])
}

/// The room the blade needs in a foot, in the cell frame of a part seated on the plate: its
/// way from the lifted position down past the seat, with `gap` round it and room to bend
/// away on its back. It runs straight out of the wall, so the way in is the same slot.
fn blade_slot(gap: f64) -> Solid {
    let x0 = -DETENT_T / 2.0 - DETENT_ROOM;
    let x1 = DETENT_T / 2.0 + DETENT_NOSE + gap;
    let y0 = DETENT_Y0 - HOOK_LIFT - HOOK_OVERTRAVEL - gap;
    let y1 = DETENT_Y0 + DETENT_LEN + BIN_GAP + gap;
    cuboid([x1 - x0, y1 - y0, DETENT_H + gap + 1.0], [x0, y0, -1.0])
}

/// The rib on the slot's wall that catches the blade's nose: just below the nose when the part
/// is seated, standing `detent` into the nose's way. Coming down, its 45° flank pushes the
/// blade aside until the nose snaps over it; lifting, its steep flank meets the nose's.
fn latch_rib(gap: f64, detent: f64) -> Solid {
    let wall = DETENT_T / 2.0 + DETENT_NOSE + gap;
    let inner = DETENT_T / 2.0 + DETENT_NOSE - detent;
    let depth = wall - inner;
    // The steep flanks of rib and nose run parallel, `gap` apart along the wall.
    let top = DETENT_Y0 + DETENT_LEN - nose_len() + flank(DETENT_NOSE + gap, true) - gap;
    let (c, d) = (top - flank(depth, true), top - flank(depth, true) - DETENT_FLAT);
    let bottom = d - flank(depth, false);
    let pts = [[wall + 1.0, top], [wall, top], [inner, c], [inner, d], [wall, bottom], [wall + 1.0, bottom]];
    translate(&polygon_prism(&pts, 0.0, DETENT_H + gap + 2.0, 8), [0.0, 0.0, -1.0])
}

/// Which feet carry the latch rib: the lowest row, at most the two outer ones, so a big part
/// does not sit too tight.
fn latched(i: u32, j: u32, cells: [u32; 2]) -> bool {
    j == 0 && (i == 0 || i + 1 == cells[0])
}

/// From the frame of cell (i, j) on the plate into the print frame of a part `depth` deep.
fn from_cell(s: &Solid, i: u32, j: u32, depth: f64) -> Solid {
    translate(&s.rotate(90.0, 0.0, 0.0), [BIN_SIZE / 2.0 + GRID * i as f64, depth, GRID * j as f64 - BIN_GAP])
}

/// A rounded rectangle across x (width `w`) and up z (height `h`), pulled in by `inset`,
/// reaching from y0 to y1.
fn outline(w: f64, h: f64, inset: f64, y: [f64; 2], seg: u32) -> Solid {
    let s = rrect_prism(w - 2.0 * inset, h - 2.0 * inset, (BIN_R - inset).max(0.5), y[1] - y[0], seg);
    translate(&s.rotate(90.0, 0.0, 0.0), [w / 2.0, y[1], h / 2.0])
}

/// Where the magnets go, in cell coordinates: every cell, or only the outermost four.
fn magnet_cells(cells: [u32; 2], mode: MagnetMode, pos: f64) -> Vec<(u32, u32, f64, f64)> {
    let mut out = Vec::new();
    if mode == MagnetMode::None {
        return out;
    }
    for i in 0..cells[0] {
        for j in 0..cells[1] {
            for (sx, x) in [(-1.0, i == 0), (1.0, i + 1 == cells[0])] {
                for (k, y) in [(0, j == 0), (1, j + 1 == cells[1])] {
                    if mode == MagnetMode::All || (x && y) {
                        out.push((i, j, sx * pos, WALL_MAGNET_Y[k]));
                    }
                }
            }
        }
    }
    out
}

/// The solid every wall part starts from: its feet (with keyholes when `hooks`) and the body
/// from `front` to the feet, with magnet holes in the feet.
fn block(cells: [u32; 2], units: u32, hooks: bool, magnets: MagnetMode, front: f64, p: &Profile) -> Solid {
    let [w, d, h] = size(cells, units);
    let seg = p.seg;
    let foot = swept(BIN_SIZE, BIN_SIZE - HOOK_LIFT, BIN_R, &FOOT_PROFILE, (0.0, GRID / 2.0 - HOOK_LIFT / 2.0), seg);
    let gap = p.hook_gap;
    let (plain, with_rib) = if hooks {
        let plain = subtract(foot.clone(), vec![keyhole(gap), blade_slot(gap)]);
        let with_rib = if p.detent > 0.0 {
            subtract(foot, vec![keyhole(gap), blade_slot(gap) - latch_rib(gap, p.detent)])
        } else {
            plain.clone()
        };
        (plain, with_rib)
    } else {
        (foot.clone(), foot)
    };
    let mut parts = vec![outline(w, h, 0.0, [front, d - FOOT_H + 0.01], seg)];
    for i in 0..cells[0] {
        for j in 0..cells[1] {
            let foot = if latched(i, j, cells) { &with_rib } else { &plain };
            parts.push(from_cell(foot, i, j, d));
        }
    }
    let hole = magnet_hole(p);
    let holes = magnet_cells(cells, magnets, p.magnet.pos)
        .into_iter()
        .map(|(i, j, x, y)| from_cell(&translate(&hole, [x, y, 0.0]), i, j, d))
        .collect();
    subtract(union(parts), holes)
}

/// A wall bin: an open box with its compartments, things go in from above. The label text is
/// engraved into the front.
pub fn tray(b: &BinSpec, hooks: bool, p: &Profile) -> Part {
    let [w, d, h] = size(b.cells, b.units);
    let seg = p.seg;
    let back = d - FOOT_H - FLOOR;
    let mut solid = block(b.cells, b.units, hooks, b.magnets, 0.0, p);
    let open = cuboid([w - 2.0 * WALL, back - WALL, h / 2.0 + 1.0], [WALL, WALL, h / 2.0]);
    solid -= outline(w, h, WALL, [WALL, back], seg) + open;
    // Divider walls: across x by dividers[0], front to back by dividers[1].
    let mut walls = Vec::new();
    let [nx, ny] = b.dividers.map(|n| n.max(1));
    let top = h - 1.0;
    for k in 1..nx {
        let x = WALL + k as f64 * (w - 2.0 * WALL) / nx as f64;
        walls.push(cuboid([WALL, back - WALL + 0.02, top - WALL], [x - WALL / 2.0, WALL - 0.01, WALL - 0.01]));
    }
    for k in 1..ny {
        let y = WALL + k as f64 * (back - WALL) / ny as f64;
        walls.push(cuboid([w - 2.0 * WALL + 0.02, WALL, top - WALL], [WALL - 0.01, y - WALL / 2.0, WALL - 0.01]));
    }
    if !walls.is_empty() {
        solid = union(std::iter::once(solid).chain(walls).collect());
    }
    // An open front: the front wall only up to a little under half the height.
    let z_open = (h * 0.45).max(WALL + 6.0);
    // A hopper: the front low, the sides (and dividers) cut on a slope up towards the wall, a
    // round bead along the inside of the front's top edge.
    let z_front = (h * HOPPER_FRONT).max(WALL + 8.0);
    if b.hopper {
        let y_full = back * HOPPER_SLOPE;
        let rise = (h - z_front) / y_full;
        let slope = [[-1.0, z_front - rise], [y_full, h], [y_full, h + 1.0], [-1.0, h + 1.0]];
        solid -= translate(&across(&slope, w / 2.0 + 1.0), [w / 2.0, 0.0, 0.0]);
        let bead = cylinder(w - 2.0 * BIN_R, WALL, WALL, seg).rotate(0.0, 90.0, 0.0);
        solid += translate(&bead, [BIN_R, WALL, z_front]);
    } else if b.open_front {
        solid -= cuboid([w - 2.0 * BIN_R, WALL + 1.02, h], [BIN_R, -1.0, z_open]);
    }
    if let Some(text) = b.label_text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let wall_top = if b.hopper {
            z_front
        } else if b.open_front {
            z_open
        } else {
            h
        };
        let height = super::text::fit_height(text, (wall_top - WALL - 3.0).min(6.0), w - 2.0 * WALL - 4.0);
        if height >= 2.0 {
            solid =
                super::text::engrave_front(solid, text, [w / 2.0, 0.0, WALL + (wall_top - WALL) / 2.0], height, 0.5);
        }
    }
    Part::new(crate::names::wall_bin(b, hooks), solid)
}

/// A block of one cell with the hook's keyhole cut for `gap`, to find the gap that hangs well:
/// two units deep, the gap engraved on the front.
pub fn hook_test(gap: f64, p: &Profile) -> Part {
    test_block(
        &Profile { hook_gap: gap, ..p.clone() },
        &format!("{gap:.1}"),
        format!("kalibrierung_wand_haken_{gap:.1}.stl"),
    )
}

/// A block of one cell whose latch rib stands `detent` into the blade's way, to find how firmly
/// parts should click in: the value engraved on the front after an R.
pub fn latch_test(detent: f64, p: &Profile) -> Part {
    let name = format!("kalibrierung_wand_raste_{detent:.1}.stl");
    test_block(&Profile { detent, ..p.clone() }, &format!("R{detent:.1}"), name)
}

fn test_block(profile: &Profile, text: &str, name: String) -> Part {
    let [w, _, h] = size([1, 1], 2);
    let block = block([1, 1], 2, true, MagnetMode::None, 0.0, profile);
    Part::new(name, super::text::engrave_front(block, text, [w / 2.0, 0.0, h / 2.0], 6.0, 0.6))
}

/// How deep text goes into a face.
const ENGRAVE: f64 = 0.5;

/// What a wall holder is built in: its size, the back plate's front face (things stand in
/// front of it, y from 0 to there) and the back plate with feet.
struct Frame {
    w: f64,
    h: f64,
    back: f64,
    solid: Solid,
}

fn frame(cells: [u32; 2], units: u32, hooks: bool, magnets: MagnetMode, p: &Profile) -> Frame {
    let [w, d, h] = size(cells, units);
    let back = d - FOOT_H - FLOOR;
    Frame { w, h, back, solid: block(cells, units, hooks, magnets, back, p) }
}

impl Frame {
    /// A block standing out of the back plate from its bottom: x from `x` over `width`, up to
    /// `top`, with the part's rounded corners below.
    fn shelf(&self, x: [f64; 2], top: f64, seg: u32) -> Solid {
        let top = top.min(self.h);
        let out = outline(self.w, self.h, 0.0, [0.0, self.back + 0.01], seg);
        out ^ cuboid([x[1] - x[0], self.back + 2.0, top + 1.0], [x[0], -1.0, -1.0])
    }

    /// Text on the back plate's face above `z`, centred on `x`, as large as `width` allows.
    fn above(&self, solid: Solid, text: &str, x: f64, z: f64, width: f64) -> Solid {
        let room = self.h - z - 2.0;
        let height = text::fit_height(text, room.min(6.0), width);
        if height < 2.0 {
            return solid;
        }
        text::engrave_front(solid, text, [x, self.back, z + 1.0 + height / 2.0], height, ENGRAVE)
    }
}

/// A wall holder; `magnets` in its feet.
pub fn holder(h: &HolderSpec, hooks: bool, magnets: MagnetMode, p: &Profile) -> Part {
    let units = h.units();
    let solid = match h {
        HolderSpec::WallHoles(s) => wall_holes(s, units, hooks, magnets, p),
        HolderSpec::WallCradles(s) => wall_cradles(s, units, hooks, magnets, p),
        HolderSpec::WallVSlots(s) => wall_vslots(s, units, hooks, magnets, p),
        other => panic!("not a wall holder: {other:?}"),
    };
    Part::new(crate::names::wall_holder(h, hooks), solid)
}

fn title(t: &Option<String>) -> Option<&str> {
    t.as_deref().map(str::trim).filter(|t| !t.is_empty())
}

/// A block at the bottom with a grid of holes from above, leaning away from the wall.
fn wall_holes(s: &WallHolesSpec, units: u32, hooks: bool, magnets: MagnetMode, p: &Profile) -> Solid {
    let f = frame(s.cells, units, hooks, magnets, p);
    let [_, _, top] = s.block();
    let top = top.min(f.h - 1.0);
    let shelf = f.shelf([0.0, f.w], top, p.seg);
    let [nx, ny] = s.count.map(|c| c.max(1));
    let (pitch, a) = (s.pitch(), s.hole());
    let tilt = s.tilt.clamp(0.0, 30.0);
    let lean = s.depth * tilt.to_radians().sin();
    // Rows from the front, the whole pattern (with the lean) centred in front of the plate.
    let span = (ny as f64 - 1.0) * pitch + a + lean;
    let y0 = (f.back - span) / 2.0 + a / 2.0;
    let x0 = f.w / 2.0 - (nx as f64 - 1.0) * pitch / 2.0;
    let depth = if s.through { top + 2.0 } else { s.depth.min(top - FLOOR) };
    let hole = hole_solid(s.shape, s.d + s.clearance, depth, 0.8, p.seg);
    let hole = translate(&hole, [0.0, 0.0, -depth]).rotate(tilt, 0.0, 0.0);
    let cuts: Vec<Solid> = (0..nx)
        .flat_map(|i| (0..ny).map(move |j| (i, j)))
        .map(|(i, j)| translate(&hole, [x0 + i as f64 * pitch, y0 + j as f64 * pitch, top]))
        .collect();
    let mut solid = union(vec![f.solid.clone(), subtract(shelf, cuts)]);
    if let Some(t) = title(&s.title) {
        solid = f.above(solid, t, f.w / 2.0, top, f.w - 2.0 * WALL - 4.0);
    }
    solid
}

/// A column per place standing out of the back plate, each a step higher when stepped, with a
/// socket leaning towards the wall at the top and its text on the front.
fn wall_cradles(s: &WallCradleSpec, units: u32, hooks: bool, magnets: MagnetMode, p: &Profile) -> Solid {
    let f = frame(s.cells, units, hooks, magnets, p);
    let [width, _, _] = s.block();
    let angle = s.angle.clamp(0.0, 30.0);
    let mut x = f.w / 2.0 - width / 2.0;
    let mut columns = Vec::new();
    let mut cuts = Vec::new();
    let mut labels = Vec::new();
    for (k, slot) in s.slots.iter().enumerate() {
        let place = s.place(slot);
        let top = s.top(k).min(f.h - 1.0);
        // The outer columns reach to the part's sides.
        let x_from = if k == 0 { 0.0 } else { x };
        let x_to = if k + 1 == s.slots.len() { f.w } else { x + place };
        columns.push(f.shelf([x_from, x_to], top, p.seg));
        let r = (slot.d + s.clearance) / 2.0;
        let depth = slot.depth().min(top - FLOOR - 0.5);
        let yc = f.back - HOLE_WALL - r - 0.5;
        let cx = x + HOLE_WALL + r;
        let socket =
            cylinder(depth + 1.0, r, r, p.seg) + translate(&cylinder(1.0, r, r + 1.0, p.seg), [0.0, 0.0, depth - 0.99]);
        cuts.push(translate(&translate(&socket, [0.0, 0.0, -depth]).rotate(-angle, 0.0, 0.0), [cx, yc, top]));
        let text = slot.label.trim();
        if !text.is_empty() {
            let height = text::fit_height(text, 5.0, place - 1.0).min(top - 4.0);
            if height >= 2.0 {
                labels.push((text.to_string(), [cx, 0.0, top - 2.0 - height / 2.0], height));
            }
        }
        x += place;
    }
    let body = subtract(union(columns), cuts);
    let mut solid = union(vec![f.solid.clone(), body]);
    for (t, at, height) in labels {
        solid = text::engrave_front(solid, &t, at, height, ENGRAVE);
    }
    if let Some(t) = title(&s.title) {
        let top = s.top(s.slots.len().saturating_sub(1)).min(f.h - 1.0);
        solid = f.above(solid, t, f.w / 2.0, top, f.w - 2.0 * WALL - 4.0);
    }
    solid
}

/// A shelf out of the back plate with V-bottomed slots running out from the wall, open at the
/// front unless closed, things standing in them jaws down; their texts above on the plate.
fn wall_vslots(s: &WallVSlotSpec, units: u32, hooks: bool, magnets: MagnetMode, p: &Profile) -> Solid {
    let f = frame(s.cells, units, hooks, magnets, p);
    let [width, _, top] = s.block();
    let top = top.min(f.h - 1.0);
    let shelf = f.shelf([0.0, f.w], top, p.seg);
    let (y0, y1) = (if s.closed_front { WALL } else { -1.0 }, f.back + 0.01);
    let mut x = f.w / 2.0 - width / 2.0 + HOLE_WALL;
    let mut cuts = Vec::new();
    let mut labels = Vec::new();
    for slot in &s.slots {
        let wd = slot.width + s.clearance;
        let v = wd / 2.0;
        let depth = slot.depth.min(top - FLOOR).max(1.0);
        let profile: Vec<[f64; 2]> = vec![[0.0, -depth], [v, -depth + v], [v, 50.0], [-v, 50.0], [-v, -depth + v]];
        // The prism lies along y from -(y1 - y0) to 0 after the turn.
        let prism = polygon_prism(&profile, 0.0, y1 - y0, 8).rotate(90.0, 0.0, 0.0);
        cuts.push(translate(&prism, [x + v, y1, top]));
        if !slot.label.trim().is_empty() {
            labels.push((slot.label.trim().to_string(), x + v, wd + HOLE_WALL - 1.0));
        }
        x += wd + HOLE_WALL;
    }
    let mut solid = union(vec![f.solid.clone(), subtract(shelf, cuts)]);
    for (t, cx, room) in labels {
        solid = f.above(solid, &t, cx, top, room);
    }
    if let Some(t) = title(&s.title) {
        // Above the slots' texts when there are any.
        let z = if s.slots.iter().any(|v| !v.label.trim().is_empty()) { top + 9.0 } else { top };
        solid = f.above(solid, t, f.w / 2.0, z, f.w - 2.0 * WALL - 4.0);
    }
    solid
}

/// A hook that hangs a wall plate on an IKEA Skadis board: its dovetail tab goes into a pocket
/// in the plate's top edge from behind, the bar through a slot of the board and the leg down
/// behind it. Made lying on its side.
pub fn skadis_hook(p: &Profile) -> Part {
    // Lying on its side: x becomes z.
    let hook = skadis_hook_at(0.0, 0.0, p).rotate(0.0, -90.0, 0.0);
    let lo = hook.bounding_box().min;
    Part::new("skadis_haken.stl".into(), translate(&hook, [-lo.x, -lo.y, -lo.z]))
}

/// A Skadis hook in plate coordinates, its tab centred on `x` in the top edge at `top`.
pub fn skadis_hook_at(x: f64, top: f64, p: &Profile) -> Solid {
    let fl = p.magnet.h + p.magnet.floor;
    let tab = super::baseplate::dovetail_prism(&super::baseplate::top_tail(0.0, 0.0, 0.0), fl, false);
    let n = CONNECTOR_NECK / 2.0;
    let w = SKADIS_W / 2.0;
    let behind = SKADIS_BOARD + SKADIS_AIR;
    let strip = cuboid([2.0 * n, SKADIS_STEM, SKADIS_STRIP], [-n, 0.0, 0.0]);
    let bar = cuboid([2.0 * w, SKADIS_BAR, behind + 0.01], [-w, SKADIS_STEM - SKADIS_BAR, -behind]);
    let leg = cuboid(
        [2.0 * w, SKADIS_BAR + SKADIS_LEG, SKADIS_T],
        [-w, SKADIS_STEM - SKADIS_BAR - SKADIS_LEG, -behind - SKADIS_T],
    );
    translate(&union(vec![tab, strip, bar, leg]), [x, top, 0.0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hook_stands_where_its_constants_say() {
        let b = hook().bounding_box();
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        assert!(near(b.min.x, -HOOK_W / 2.0) && near(b.max.x, HOOK_W / 2.0), "{b:?}");
        assert!(near(b.min.y, HOOK_Y0) && near(b.max.y, HOOK_Y0 + HOOK_REACH), "{b:?}");
        assert!(near(b.min.z, -0.01) && near(b.max.z, HOOK_H), "{b:?}");
        // Stem, gusset and finger, nothing else.
        let side = HOOK_STEM * HOOK_H
            + HOOK_GUSSET * HOOK_FINGER
            + HOOK_GUSSET * HOOK_GUSSET / 2.0
            + (HOOK_REACH - HOOK_STEM - HOOK_GUSSET) * HOOK_FINGER
            - 0.5 * 0.5 / 2.0
            + HOOK_STEM * 0.01;
        assert!((hook().volume() - side * HOOK_W).abs() < 1e-3, "{} vs {}", hook().volume(), side * HOOK_W);
    }

    #[test]
    fn the_keyhole_keeps_clear_of_the_foot_flanks() {
        // The foot of a cell and the same foot pulled in by 1.2 mm all round: between them the
        // flanks, which the keyhole must not touch (it opens only through the underside).
        let foot = |inset: f64| {
            let (w, h) = (BIN_SIZE - 2.0 * inset, BIN_SIZE - HOOK_LIFT - 2.0 * inset);
            swept(w, h, BIN_R - inset, &FOOT_PROFILE, (0.0, GRID / 2.0 - HOOK_LIFT / 2.0), 32)
        };
        let flanks = foot(0.0) - foot(1.2);
        for gap in [0.2, 0.3, 0.4] {
            let hit = (&(keyhole(gap) + blade_slot(gap)) ^ &flanks).volume();
            assert!(hit < 0.01, "gap {gap}: keyhole and slot cut {hit:.3} mm³ out of the foot's flanks");
        }
    }

    #[test]
    fn hull_of_a_square_and_a_point_inside() {
        let h = hull2d(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]]);
        assert_eq!(h, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    }
}
