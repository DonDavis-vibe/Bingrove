//! The baseplate: an open grid frame that fills the drawer, cut along grid lines into pieces
//! that fit the printer. At the cuts the pieces complete each other to the full grid. In ToFloor
//! edge zones the plate ends just past the outer grid line where edge bins take over.

use super::Part;
use super::kernel::{
    Solid, Vec3, cuboid, cuboid_centred, cylinder, polygon_prism, rrect_prism, subtract, swept, translate, union,
};
use crate::grid::Grid;
use crate::model::{Mount, PlateBuild, PlateFloor, PlateSpec, PlateStyle, Side};
use crate::plan::{EdgeClaims, PlatePiece};
use crate::profile::Profile;
use crate::spec::*;

/// The whole plate in plate coordinates: front-left corner at the origin, bottom at z = 0.
pub fn plate(grid: &Grid, claims: &EdgeClaims, build: PlateBuild, p: &Profile) -> Solid {
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
            if build.style == PlateStyle::Skeleton {
                cuts.push(translate(&cross, [cx, cy, fl / 2.0]));
            }
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    let (x, y) = (cx + sx * m.pos, cy + sy * m.pos);
                    if build.floor == PlateFloor::Magnets {
                        cuts.push(translate(&hole, [x, y, fl - m.h]));
                    }
                    if build.magnets_below {
                        cuts.push(translate(&hole, [x, y, -1.0]));
                    }
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

/// The plate cut into `pieces`, each moved to the origin. With a floor, `mount` adds
/// countersunk screw holes at the outer corner pads of every piece and dovetail connectors along
/// the cut lines: a tab on the left (front) piece, its pocket in the right (back) one.
pub fn pieces_of(grid: &Grid, claims: &EdgeClaims, pieces: &[PlatePiece], build: PlateBuild, p: &Profile) -> Vec<Part> {
    let whole = plate(grid, claims, build, p);
    let h = grid.height();
    let fl = grid.floor;
    let mount = if fl > 0.0 { build.mount } else { Mount::default() };
    let has = |row: usize, col: usize| pieces.iter().any(|q| q.row == row && q.col == col);
    pieces
        .iter()
        .map(|piece| {
            let [x0, y0, x1, y1] = piece.rect;
            let cut = cuboid([x1 - x0, y1 - y0, h + 2.0], [x0, y0, -1.0]);
            let mut solid = &whole ^ &cut;
            if mount.screws {
                solid = subtract(solid, screw_holes(grid, piece, fl, p));
            }
            if mount.connectors {
                let sides = [
                    has(piece.row, piece.col + 1),
                    has(piece.row + 1, piece.col),
                    piece.col > 0 && has(piece.row, piece.col - 1),
                    piece.row > 0 && has(piece.row - 1, piece.col),
                ];
                let (tabs, pockets) = connectors(grid, piece, fl, p, sides);
                solid = subtract(union(std::iter::once(solid).chain(tabs).collect()), pockets);
            }
            let lo = solid.bounding_box().min;
            let solid = solid.translate(Vec3::new(-lo.x, -lo.y, 0.0));
            let mut part = Part::new(crate::names::baseplate(piece.row, piece.col, piece.cells, build), solid);
            part.origin = [lo.x, lo.y];
            part
        })
        .collect()
}

/// Half the side of the floor pad at a cell corner: what the cross cut-out leaves standing.
fn pad(p: &Profile) -> f64 {
    let a = p.magnet.pos - p.magnet.d / 2.0 - p.magnet.wall;
    GRID / 2.0 - a
}

/// Countersunk screw holes where the piece's four outer corner cells have their outermost
/// magnet position, through the floor.
fn screw_holes(grid: &Grid, piece: &PlatePiece, fl: f64, p: &Profile) -> Vec<Solid> {
    let [fx, fy] = piece.first;
    let [cx, cy] = piece.cells;
    let (m_pos, seg) = (p.magnet.pos, p.seg);
    let sink = (SCREW_HEAD - SCREW_D) / 2.0;
    let hole = cylinder(fl + 2.0, SCREW_D / 2.0, SCREW_D / 2.0, seg)
        + translate(&cylinder(sink, SCREW_D / 2.0, SCREW_HEAD / 2.0, seg), [0.0, 0.0, fl + 1.0 - sink])
        + translate(&cylinder(1.0, SCREW_HEAD / 2.0, SCREW_HEAD / 2.0, seg), [0.0, 0.0, fl + 1.0]);
    let mut out = Vec::new();
    for (i, sx) in [(fx, -1.0), (fx + cx - 1, 1.0)] {
        for (j, sy) in [(fy, -1.0), (fy + cy - 1, 1.0)] {
            let x = grid.line(0, i) + GRID / 2.0 + sx * m_pos;
            let y = grid.line(1, j) + GRID / 2.0 + sy * m_pos;
            out.push(translate(&hole, [x, y, -1.0]));
        }
    }
    out
}

/// Dovetail tabs on the right and back cut lines of a piece (where a neighbour follows) and
/// the pockets for the neighbours' tabs on its left and front cut lines, one per two cells,
/// sitting on the corner pads of the floor. `sides`: a neighbour to the right, back, left, front.
fn connectors(grid: &Grid, piece: &PlatePiece, fl: f64, p: &Profile, sides: [bool; 4]) -> (Vec<Solid>, Vec<Solid>) {
    let [right, back, left, front] = sides;
    let [fx, fy] = piece.first;
    let [cx, cy] = piece.cells;
    let z0 = CONNECTOR_INSET;
    let height = fl - 2.0 * CONNECTOR_INSET;
    let pad_centre = pad(p) / 2.0;
    let (gap, big) = (CONNECTOR_GAP, 1.0);
    // A dovetail reaching from the line at `at` along `axis` into +axis, centred on `along`.
    let tail = |axis: usize, at: f64, along: f64, grow: f64| -> Solid {
        let (n, hd, d) = (CONNECTOR_NECK / 2.0 + grow, CONNECTOR_HEAD / 2.0 + grow, CONNECTOR_DEPTH + grow);
        let pts: Vec<[f64; 2]> = if axis == 0 {
            vec![[at - big, along - n], [at + d, along - hd], [at + d, along + hd], [at - big, along + n]]
        } else {
            vec![[along - n, at - big], [along + n, at - big], [along + hd, at + d], [along - hd, at + d]]
        };
        translate(&polygon_prism(&pts, 0.0, height, 8), [0.0, 0.0, z0])
    };
    let mut tabs = Vec::new();
    let mut pockets = Vec::new();
    // Along a vertical line (x fixed): tabs centred on the front pad of every other cell row.
    let ys: Vec<f64> = (fy..fy + cy).step_by(2).map(|k| grid.line(1, k) + pad_centre).collect();
    let xs: Vec<f64> = (fx..fx + cx).step_by(2).map(|k| grid.line(0, k) + pad_centre).collect();
    if right {
        tabs.extend(ys.iter().map(|&y| tail(0, grid.line(0, fx + cx), y, 0.0)));
    }
    if back {
        tabs.extend(xs.iter().map(|&x| tail(1, grid.line(1, fy + cy), x, 0.0)));
    }
    if left {
        pockets.extend(ys.iter().map(|&y| tail(0, grid.line(0, fx), y, gap)));
    }
    if front {
        pockets.extend(xs.iter().map(|&x| tail(1, grid.line(1, fy), x, gap)));
    }
    (tabs, pockets)
}

/// The original script's plate: centred grid, solid margins, the given split.
pub fn pieces(spec: &PlateSpec, p: &Profile) -> Vec<Part> {
    let (grid, _) = Grid::new(spec.drawer, &spec.options(), &p.magnet).expect("plate spec fits its drawer");
    let claims = EdgeClaims { sides: Default::default(), corners: [false; 4] };
    let pieces = crate::plan::plate_pieces(&grid, grid.margins, &spec.split);
    let floor = if spec.magnets { PlateFloor::Magnets } else { PlateFloor::Open };
    pieces_of(&grid, &claims, &pieces, PlateBuild { floor, ..PlateBuild::default() }, p)
}
