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
use super::kernel::{Solid, cuboid, rrect_prism, subtract, swept, translate, union};
use crate::model::{BinSpec, MagnetMode};
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

/// The solid every wall part starts from: its feet (with keyholes when `hooks`) and the whole
/// body from the front to the feet, with magnet holes in the feet.
fn block(cells: [u32; 2], units: u32, hooks: bool, magnets: MagnetMode, p: &Profile) -> Solid {
    let [w, d, h] = size(cells, units);
    let seg = p.seg;
    let foot = swept(BIN_SIZE, BIN_SIZE - HOOK_LIFT, BIN_R, &FOOT_PROFILE, (0.0, GRID / 2.0 - HOOK_LIFT / 2.0), seg);
    let foot = if hooks { foot - keyhole(p.hook_gap) } else { foot };
    let mut parts = vec![outline(w, h, 0.0, [0.0, d - FOOT_H + 0.01], seg)];
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
    let mut solid = block(b.cells, b.units, hooks, b.magnets, p);
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
        block([1, 1], 2, true, MagnetMode::None, &profile),
        &text,
        [w / 2.0, 0.0, h / 2.0],
        6.0,
        0.6,
    );
    Part::new(format!("kalibrierung_wand_haken_{gap:.1}.stl"), solid)
}
