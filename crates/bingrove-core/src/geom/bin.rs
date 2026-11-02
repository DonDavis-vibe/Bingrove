//! Bins: a foot per cell, the body, the stacking lip, dividers, magnet holes, label shelf and
//! finger scoop. Edge bins reach past their cells into the drawer's margins; in ToFloor mode a
//! skirt carries the overhang down to the drawer floor and the floor drops there.

use super::Part;
use super::kernel::{Solid, Vec3, cuboid, cylinder, rrect_prism, subtract, swept, translate, union};
use crate::model::{BinSpec, DividerMode, EdgeMode, MagnetMode};
use crate::profile::Profile;
use crate::spec::*;

/// Centre of cell (i, j) relative to the centre of a bin of `cells`.
pub(crate) fn cell_centre(cells: [u32; 2], i: u32, j: u32) -> (f64, f64) {
    ((i as f64 - (cells[0] as f64 - 1.0) / 2.0) * GRID, (j as f64 - (cells[1] as f64 - 1.0) / 2.0) * GRID)
}

/// One foot per cell, centred on the bin.
pub(crate) fn feet(cells: [u32; 2], seg: u32) -> Vec<Solid> {
    (0..cells[0])
        .flat_map(|i| (0..cells[1]).map(move |j| (i, j)))
        .map(|(i, j)| swept(BIN_SIZE, BIN_SIZE, BIN_R, &FOOT_PROFILE, cell_centre(cells, i, j), seg))
        .collect()
}

/// Magnet hole positions from below, centred on the bin.
pub(crate) fn magnet_points(cells: [u32; 2], mode: MagnetMode, pos: f64) -> Vec<(f64, f64)> {
    if mode == MagnetMode::None {
        return vec![];
    }
    let mut pts = Vec::new();
    for i in 0..cells[0] {
        for j in 0..cells[1] {
            let (cx, cy) = cell_centre(cells, i, j);
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    pts.push((cx + sx * pos, cy + sy * pos));
                }
            }
        }
    }
    if mode == MagnetMode::Corners {
        let mx = pts.iter().map(|p| p.0.abs()).fold(0.0, f64::max);
        let my = pts.iter().map(|p| p.1.abs()).fold(0.0, f64::max);
        pts.retain(|p| (p.0.abs() - mx).abs() < 1e-9 && (p.1.abs() - my).abs() < 1e-9);
    }
    pts
}

/// Magnet hole cut from below, with a chamfer against elephant's foot.
pub(crate) fn magnet_hole(p: &Profile) -> Solid {
    let m = &p.magnet;
    let r = m.d / 2.0;
    let hole = cylinder(m.h + 1.0, r, r, p.seg) + cylinder(m.chamfer + 1.0, r + m.chamfer + 1.0, r, p.seg);
    translate(&hole, [0.0, 0.0, -1.0])
}

/// A box between two corners.
fn span(x: [f64; 2], y: [f64; 2], z: [f64; 2]) -> Solid {
    cuboid([x[1] - x[0], y[1] - y[0], z[1] - z[0]], [x[0], y[0], z[0]])
}

/// A standard bin, minimum corner at the origin.
pub fn bin(b: &BinSpec, p: &Profile) -> Part {
    edge_bin(b, [0.0; 4], EdgeMode::default(), 1.0, p)
}

/// A bin reaching `e` (left, right, front, back) past its cells; `rim` is how far the plate
/// keeps its top past the outer grid line in ToFloor mode. Minimum corner at the origin.
pub fn edge_bin(b: &BinSpec, e: [f64; 4], mode: EdgeMode, rim: f64, p: &Profile) -> Part {
    let seg = p.seg;
    let (w, d) = (bin_outer(b.cells[0]), bin_outer(b.cells[1]));
    let h_body = body_height(b.units);
    let top = h_body + if b.lip { LIP_H } else { 0.0 };
    // Outline in bin coordinates (centre of the cells at the origin).
    let (x0, x1, y0, y1) = (-w / 2.0 - e[0], w / 2.0 + e[1], -d / 2.0 - e[2], d / 2.0 + e[3]);
    let (ow, od) = (x1 - x0, y1 - y0);
    let c = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let at = |s: Solid, z: f64| translate(&s, [c.0, c.1, z]);

    let mut parts = feet(b.cells, seg);
    parts.push(at(rrect_prism(ow, od, BIN_R, top - FOOT_H, seg), FOOT_H));

    // With a lip, the cavity follows the baseplate profile at the top, with a 45° support down to the wall.
    let cavity_profile: Vec<(f64, f64)> = if b.lip {
        vec![
            (top + 1.0, -1.0),
            (top, 0.0),
            (top - 1.9, 1.9),
            (h_body + 0.7, 1.9),
            (h_body, 2.6),
            (h_body - 2.6 + WALL, WALL),
            (FLOOR_Z, WALL),
        ]
    } else {
        vec![(top + 1.0, WALL), (FLOOR_Z, WALL)]
    };
    let mut cuts = vec![swept(ow, od, BIN_R, &cavity_profile, c, seg)];

    // ToFloor: a skirt from the drawer floor under the overhang, outside the plate's rim, and
    // the floor dropped to FLOOR inside it. `inner[s]` is where the skirt begins on side s.
    let grid_line = [-w / 2.0 - BIN_GAP, w / 2.0 + BIN_GAP, -d / 2.0 - BIN_GAP, d / 2.0 + BIN_GAP];
    let skirt_inner = [0usize, 1, 2, 3].map(|s| {
        let out = if s.is_multiple_of(2) { -1.0 } else { 1.0 };
        grid_line[s] + out * (rim + SKIRT_GAP)
    });
    let skirts: Vec<usize> =
        if mode == EdgeMode::ToFloor { (0..4).filter(|&s| e[s] > rim + SKIRT_GAP).collect() } else { vec![] };
    if !skirts.is_empty() {
        let big = 1.0;
        let reach = |s: usize, wall: f64| -> Solid {
            let (xr, yr) = match s {
                0 => ([x0 - big, skirt_inner[0] - wall], [y0 - big, y1 + big]),
                1 => ([skirt_inner[1] + wall, x1 + big], [y0 - big, y1 + big]),
                2 => ([x0 - big, x1 + big], [y0 - big, skirt_inner[2] - wall]),
                _ => ([x0 - big, x1 + big], [skirt_inner[3] + wall, y1 + big]),
            };
            span(xr, yr, [-big, top + big])
        };
        let outer_low = at(rrect_prism(ow, od, BIN_R, FOOT_H + 0.02, seg), 0.0);
        let skirt = union(skirts.iter().map(|&s| reach(s, 0.0)).collect()) ^ outer_low;
        parts.push(skirt);
        let inner = at(rrect_prism(ow - 2.0 * WALL, od - 2.0 * WALL, BIN_R - WALL, FLOOR_Z + 0.1 - FLOOR, seg), FLOOR);
        cuts.push(union(skirts.iter().map(|&s| reach(s, WALL)).collect()) ^ inner);
    }
    let floor_at = |s: usize| if skirts.contains(&s) { FLOOR } else { FLOOR_Z };
    let mut solid = union(parts) - union(cuts);
    // Where the floor is dropped (only there may dividers reach below the core floor).
    let dropped = (!skirts.is_empty()).then(|| {
        union(
            skirts
                .iter()
                .map(|&s| match s {
                    0 => span([x0 - 1.0, skirt_inner[0]], [y0 - 1.0, y1 + 1.0], [FLOOR - 0.02, FLOOR_Z + 0.01]),
                    1 => span([skirt_inner[1], x1 + 1.0], [y0 - 1.0, y1 + 1.0], [FLOOR - 0.02, FLOOR_Z + 0.01]),
                    2 => span([x0 - 1.0, x1 + 1.0], [y0 - 1.0, skirt_inner[2]], [FLOOR - 0.02, FLOOR_Z + 0.01]),
                    _ => span([x0 - 1.0, x1 + 1.0], [skirt_inner[3], y1 + 1.0], [FLOOR - 0.02, FLOOR_Z + 0.01]),
                })
                .collect(),
        )
    });

    // Dividers end below the lip so bins still stack; over a dropped floor they go down to it.
    let z_div = FLOOR_Z - 0.01;
    let h_div = h_body - DIVIDER_GAP - z_div;
    let (xs, ys): (Vec<f64>, Vec<f64>) = match b.divider_mode {
        DividerMode::Equal => (
            (1..b.dividers[0]).map(|k| x0 + WALL + k as f64 * (ow - 2.0 * WALL) / b.dividers[0] as f64).collect(),
            (1..b.dividers[1]).map(|k| y0 + WALL + k as f64 * (od - 2.0 * WALL) / b.dividers[1] as f64).collect(),
        ),
        DividerMode::Grid => {
            // Inner grid lines, plus where the margin part begins.
            let boundary = |s: usize| {
                let out = if s.is_multiple_of(2) { -1.0 } else { 1.0 };
                if skirts.contains(&s) { skirt_inner[s] - out * WALL / 2.0 } else { grid_line[s] }
            };
            let mut xs: Vec<f64> = (1..b.cells[0]).map(|k| grid_line[0] + k as f64 * GRID).collect();
            let mut ys: Vec<f64> = (1..b.cells[1]).map(|k| grid_line[2] + k as f64 * GRID).collect();
            xs.extend((0..2).filter(|&s| e[s] > 0.0).map(boundary));
            ys.extend((2..4).filter(|&s| e[s] > 0.0).map(boundary));
            (xs, ys)
        }
    };
    let mut add = Vec::new();
    let mut low = Vec::new();
    let (wy, wx) =
        ([c.1 - (od - WALL) / 2.0, c.1 + (od - WALL) / 2.0], [c.0 - (ow - WALL) / 2.0, c.0 + (ow - WALL) / 2.0]);
    for x in xs {
        add.push(span([x - WALL / 2.0, x + WALL / 2.0], wy, [z_div, z_div + h_div]));
        low.push(span([x - WALL / 2.0, x + WALL / 2.0], wy, [FLOOR - 0.01, z_div + 0.01]));
    }
    for y in ys {
        add.push(span(wx, [y - WALL / 2.0, y + WALL / 2.0], [z_div, z_div + h_div]));
        low.push(span(wx, [y - WALL / 2.0, y + WALL / 2.0], [FLOOR - 0.01, z_div + 0.01]));
    }
    if let Some(dropped) = &dropped
        && !low.is_empty()
    {
        add.push(union(low) ^ dropped);
    }

    // Label shelf along the back wall, top flush with the body, on a 36° support.
    if b.label_tab {
        let tan = LABEL_ANGLE.to_radians().tan();
        let depth = LABEL_DEPTH.min((h_body - floor_at(3) - 1.0) / tan).min((od - 2.0 * WALL) / 2.0);
        if depth >= 3.0 {
            let yw = y1 - WALL + 0.01;
            let pts: Vec<Vec3> = [x0 + WALL - 0.01, x1 - WALL + 0.01]
                .into_iter()
                .flat_map(|x| {
                    [Vec3::new(x, yw, h_body), Vec3::new(x, yw - depth, h_body), Vec3::new(x, yw, h_body - depth * tan)]
                })
                .collect();
            add.push(Solid::hull(&pts));
        }
    }

    // Finger scoop: a concave fillet along the front wall.
    if b.scoop > 0 {
        let zf = floor_at(2);
        let r = (b.scoop.min(100) as f64 / 100.0 * (h_body - zf) / 2.0).min((od - 2.0 * WALL) / 3.0);
        if r >= 2.0 {
            let yf = y0 + WALL;
            let fill = span([x0 + WALL - 0.01, x1 - WALL + 0.01], [yf - 0.01, yf + r], [zf - 0.01, zf + r]);
            let round = cylinder(ow + 2.0, r, r, seg * 2).rotate(0.0, 90.0, 0.0);
            add.push(fill - translate(&round, [x0 - 1.0, yf + r, zf + r]));
        }
    }
    if !add.is_empty() {
        add.insert(0, solid);
        solid = union(add);
    }

    let hole = magnet_hole(p);
    let holes = magnet_points(b.cells, b.magnets, p.magnet.pos)
        .into_iter()
        .map(|(x, y)| translate(&hole, [x, y, 0.0]))
        .collect();
    solid = subtract(solid, holes);

    let mut solid = translate(&solid, [-x0, -y0, 0.0]);
    // The engraving: on the label shelf, read from the front, or on the front wall outside.
    if let Some(text) = b.label_text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let shelf = b
            .label_tab
            .then(|| {
                let tan = LABEL_ANGLE.to_radians().tan();
                LABEL_DEPTH.min((h_body - floor_at(3) - 1.0) / tan).min((od - 2.0 * WALL) / 2.0)
            })
            .filter(|&depth| depth >= 3.0);
        let room_x = ow - 2.0 * WALL - 4.0;
        solid = match shelf {
            Some(depth) => {
                let height = super::text::fit_height(text, (depth - 2.5).min(5.0), room_x);
                super::text::engrave(solid, text, [ow / 2.0, od - WALL - depth / 2.0, h_body], height.max(2.0), 0.6)
            }
            None => {
                let height = super::text::fit_height(text, (h_body - FLOOR_Z - 3.0).min(6.0), room_x);
                let z = FLOOR_Z + (h_body - FLOOR_Z) / 2.0;
                super::text::engrave_front(solid, text, [ow / 2.0, 0.0, z], height.max(2.0), 0.5)
            }
        };
    }
    if !b.cutouts.is_empty() {
        let top = super::insert::insert_top(&b.cutouts, h_body - DIVIDER_GAP);
        let cuts = b.cutouts.iter().map(|c| super::insert::cutout_solid(c, top, seg)).collect();
        solid = super::insert::with_pockets(solid, ow, od, top, cuts, seg);
    }
    let name = crate::names::edge_bin(b, e, mode);
    Part::new(name, solid)
}
