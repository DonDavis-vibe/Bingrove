//! The baseplate: an open grid frame that fills the drawer, cut along grid lines into pieces
//! that fit the printer. At the cuts the pieces complete each other to the full grid. In ToFloor
//! edge zones the plate ends just past the outer grid line where edge bins take over.

use super::Part;
use super::kernel::{Solid, Vec3, cuboid, cuboid_centred, cylinder, rrect_prism, subtract, swept, translate};
use crate::grid::Grid;
use crate::model::{PlateSpec, Side};
use crate::plan::{EdgeClaims, PlatePiece};
use crate::profile::Profile;
use crate::spec::*;

/// The whole plate in plate coordinates: front-left corner at the origin, bottom at z = 0.
pub fn plate(grid: &Grid, claims: &EdgeClaims, p: &Profile) -> Solid {
    let seg = p.seg;
    let m = &p.magnet;
    let [nx, ny] = grid.cells;
    let [w, d] = grid.plate;
    let fl = grid.floor;
    let h = grid.height();
    let (lx, ly) = (|i: u32| grid.line(0, i), |j: u32| grid.line(1, j));

    let centres: Vec<(f64, f64)> =
        (0..nx).flat_map(|i| (0..ny).map(move |j| (lx(i) + GRID / 2.0, ly(j) + GRID / 2.0))).collect();
    let mut cuts = Vec::new();
    if fl > 0.0 {
        // The cell cut-out ends on the floor; a cross is cut out of it, leaving a pad with one
        // magnet in each corner for the bins to rest on.
        let a = m.pos - m.d / 2.0 - m.wall;
        let arm = GRID / 2.0 - BP_PROFILE[BP_PROFILE.len() - 1].1 - 0.5;
        let cross = cuboid_centred([2.0 * a, 2.0 * arm, fl + 2.0], [0.0, 0.0, 0.0])
            + cuboid_centred([2.0 * arm, 2.0 * a, fl + 2.0], [0.0, 0.0, 0.0]);
        let hole = cylinder(m.h + 1.0, m.d / 2.0, m.d / 2.0, seg);
        for &(cx, cy) in &centres {
            let cell = swept(GRID, GRID, BP_R, &BP_PROFILE[..BP_PROFILE.len() - 1], (cx, cy), seg);
            cuts.push(translate(&cell, [0.0, 0.0, fl]));
            cuts.push(translate(&cross, [cx, cy, fl / 2.0]));
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    cuts.push(translate(&hole, [cx + sx * m.pos, cy + sy * m.pos, fl - m.h]));
                }
            }
        }
    } else {
        cuts.extend(centres.iter().map(|&c| swept(GRID, GRID, BP_R, &BP_PROFILE, c, seg)));
    }

    // Pockets from below in wide margins, one per grid cell; the ribs between them lie on the
    // grid lines and therefore also on the cut lines.
    let pw = POCKET_WALL;
    let depth = h - POCKET_TOP + 1.0;
    let mg = |s: Side| grid.margin(s);
    for (side, start) in [(Side::Front, 0.0), (Side::Back, ly(ny))] {
        if mg(side) >= POCKET_MIN {
            for i in 0..nx {
                cuts.push(cuboid([GRID - 2.0 * pw, mg(side) - 2.0 * pw, depth], [lx(i) + pw, start + pw, -1.0]));
            }
        }
    }
    for (side, start) in [(Side::Left, 0.0), (Side::Right, lx(nx))] {
        if mg(side) >= POCKET_MIN {
            for j in 0..ny {
                cuts.push(cuboid([mg(side) - 2.0 * pw, GRID - 2.0 * pw, depth], [start + pw, ly(j) + pw, -1.0]));
            }
        }
    }

    // ToFloor edge zones: away with the margin (down to the floor slab, if any) wherever an edge
    // bin takes over. The end segments run out to the plate outline, so no corner block is left
    // standing beside them.
    let (z0, zh) = if fl > 0.0 { (fl, h + 1.0 - fl) } else { (-1.0, h + 2.0) };
    let rim = grid.rim;
    for side in Side::ALL {
        if !grid.cut_back(side) {
            continue;
        }
        let along = if (side as usize) < 2 { ny } else { nx };
        for k in 0..along {
            if !claims.sides[side as usize][k as usize] {
                continue;
            }
            let first = k == 0;
            let last = k == along - 1;
            let (x0, x1, y0, y1) = match side {
                Side::Left | Side::Right => {
                    let left = side == Side::Left;
                    let (x0, x1) = if left { (-1.0, lx(0) - rim) } else { (lx(nx) + rim, w + 1.0) };
                    let y0 = if first { -1.0 } else { ly(k) };
                    let y1 = if last { d + 1.0 } else { ly(k + 1) };
                    (x0, x1, y0, y1)
                }
                Side::Front | Side::Back => {
                    let front = side == Side::Front;
                    let (y0, y1) = if front { (-1.0, ly(0) - rim) } else { (ly(ny) + rim, d + 1.0) };
                    let x0 = if first { -1.0 } else { lx(k) };
                    let x1 = if last { w + 1.0 } else { lx(k + 1) };
                    (x0, x1, y0, y1)
                }
            };
            cuts.push(cuboid([x1 - x0, y1 - y0, zh], [x0, y0, z0]));
        }
    }

    let outline = translate(&rrect_prism(w, d, CORNER_R, h, seg), [w / 2.0, d / 2.0, 0.0]);
    subtract(outline, cuts)
}

/// The plate cut into `pieces`, each moved to the origin.
pub fn pieces_of(grid: &Grid, claims: &EdgeClaims, pieces: &[PlatePiece], magnets: bool, p: &Profile) -> Vec<Part> {
    let whole = plate(grid, claims, p);
    let h = grid.height();
    pieces
        .iter()
        .map(|piece| {
            let [x0, y0, x1, y1] = piece.rect;
            let cut = cuboid([x1 - x0, y1 - y0, h + 2.0], [x0, y0, -1.0]);
            let solid = &whole ^ &cut;
            let lo = solid.bounding_box().min;
            let solid = solid.translate(Vec3::new(-lo.x, -lo.y, 0.0));
            let mut part = Part::new(crate::names::baseplate(piece.row, piece.col, piece.cells, magnets), solid);
            part.origin = [lo.x, lo.y];
            part
        })
        .collect()
}

/// The original script's plate: centred grid, solid margins, the given split.
pub fn pieces(spec: &PlateSpec, p: &Profile) -> Vec<Part> {
    let (grid, _) = Grid::new(spec.drawer, &spec.options(), &p.magnet).expect("plate spec fits its drawer");
    let claims = EdgeClaims { sides: Default::default(), corners: [false; 4] };
    let pieces = crate::plan::plate_pieces(&grid, grid.margins, &spec.split);
    pieces_of(&grid, &claims, &pieces, spec.magnets, p)
}
