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
/// sit: symmetric about the foot, which is `HOOK_LIFT` shorter at the top.
pub const WALL_MAGNET_Y: [f64; 2] = [7.5, 31.5];

pub use crate::spec::wall_size as size;

/// The hook in a cell's frame on the plate: x from the cell centre, y up the wall from the
/// cell's lower grid line, z out of the wall from the pocket floor.
pub fn hook() -> Solid {
    let x = HOOK_W / 2.0;
    let stem = cuboid([2.0 * x, HOOK_STEM, HOOK_H + 0.01], [-x, HOOK_Y0, -0.01]);
    let finger = cuboid([2.0 * x, HOOK_REACH, HOOK_FINGER], [-x, HOOK_Y0, HOOK_H - HOOK_FINGER]);
    stem + finger
}

/// The room a hook needs in a foot, in the cell frame of a part seated on the plate, with
/// `gap` all round: the hook's way from `HOOK_LIFT` lower (the part goes in lifted) up past its
/// seat by the air under the part (it settles onto its lower foot faces).
pub fn keyhole(gap: f64) -> Solid {
    let x = HOOK_W / 2.0 + gap;
    let y0 = HOOK_Y0 - HOOK_LIFT - gap;
    let way = HOOK_LIFT + BIN_GAP + 2.0 * gap;
    let stem = cuboid([2.0 * x, HOOK_STEM + way, HOOK_H + gap + 1.0], [-x, y0, -1.0]);
    let finger = cuboid([2.0 * x, HOOK_REACH + way, HOOK_FINGER + 2.0 * gap], [-x, y0, HOOK_H - HOOK_FINGER - gap]);
    stem + finger
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
    let foot = if hooks { foot - keyhole(p.hook_gap) } else { foot };
    let mut parts = vec![outline(w, h, 0.0, [front, d - FOOT_H + 0.01], seg)];
    for i in 0..cells[0] {
        for j in 0..cells[1] {
            parts.push(from_cell(&foot, i, j, d));
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
    if b.open_front {
        solid -= cuboid([w - 2.0 * BIN_R, WALL + 1.02, h], [BIN_R, -1.0, z_open]);
    }
    if let Some(text) = b.label_text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let wall_top = if b.open_front { z_open } else { h };
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
    let profile = Profile { hook_gap: gap, ..p.clone() };
    let [w, _, h] = size([1, 1], 2);
    let text = format!("{gap:.1}");
    let solid = super::text::engrave_front(
        block([1, 1], 2, true, MagnetMode::None, 0.0, &profile),
        &text,
        [w / 2.0, 0.0, h / 2.0],
        6.0,
        0.6,
    );
    Part::new(format!("kalibrierung_wand_haken_{gap:.1}.stl"), solid)
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
