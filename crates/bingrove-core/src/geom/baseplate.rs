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
                    let (x, y) = (cx + sx * m.pos, magnet_y(cy, sy, build, m.pos));
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
    let solid = subtract(outline, cuts);
    if !(build.hooks && fl > 0.0) {
        return solid;
    }
    // A wall plate's hooks, one on every pocket floor, at the cell's lower edge.
    let hook = super::wall::hook();
    let hooks = (0..nx).flat_map(|i| (0..ny).map(move |j| (i, j)));
    union(std::iter::once(solid).chain(hooks.map(|(i, j)| translate(&hook, [lx(i) + GRID / 2.0, ly(j), fl]))).collect())
}

/// Where a magnet sits along y in the cell centred on `cy`: `pos` from the centre, or on a wall
/// plate where the shorter wall parts have theirs.
fn magnet_y(cy: f64, sy: f64, build: PlateBuild, pos: f64) -> f64 {
    if build.wall { cy - GRID / 2.0 + super::wall::WALL_MAGNET_Y[usize::from(sy > 0.0)] } else { cy + sy * pos }
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
    let top_row = pieces.iter().map(|q| q.row).max().unwrap_or(0);
    pieces
        .iter()
        .map(|piece| {
            let [x0, y0, x1, y1] = piece.rect;
            let cut = cuboid([x1 - x0, y1 - y0, h + 2.0], [x0, y0, -1.0]);
            let mut solid = &whole ^ &cut;
            if mount.screws {
                solid = subtract(solid, screw_holes(grid, piece, fl, build.wall, p));
            }
            if mount.connectors {
                let sides = [
                    has(piece.row, piece.col + 1),
                    has(piece.row + 1, piece.col),
                    piece.col > 0 && has(piece.row, piece.col - 1),
                    piece.row > 0 && has(piece.row - 1, piece.col),
                ];
                let skeleton = build.style == PlateStyle::Skeleton;
                let [tabs, floor, pockets] = connectors(grid, piece, fl, skeleton, sides);
                solid = subtract(union(std::iter::once(solid).chain(tabs).chain(floor).collect()), pockets);
            }
            if build.wall && build.mount.skadis && piece.row == top_row && fl > 0.0 {
                // Pockets in the top edge for the Skadis hooks, open towards the wall.
                let top = grid.line(1, grid.cells[1]);
                let mid = (x0 + x1) / 2.0;
                let pockets = skadis_offsets(x1 - x0)
                    .into_iter()
                    .map(|dx| dovetail_prism(&top_tail(mid + dx, top, CONNECTOR_GAP), fl, true))
                    .collect();
                solid = subtract(solid, pockets);
            }
            let lo = solid.bounding_box().min;
            let solid = solid.translate(Vec3::new(-lo.x, -lo.y, 0.0));
            let mut part = Part::new(crate::names::baseplate(piece.row, piece.col, piece.cells, build), solid);
            part.origin = [lo.x, lo.y];
            part
        })
        .collect()
}

/// Countersunk screw holes at `screw_spots`, through the floor.
fn screw_holes(grid: &Grid, piece: &PlatePiece, fl: f64, wall: bool, p: &Profile) -> Vec<Solid> {
    let seg = p.seg;
    let sink = (SCREW_HEAD - SCREW_D) / 2.0;
    let hole = cylinder(fl + 2.0, SCREW_D / 2.0, SCREW_D / 2.0, seg)
        + translate(&cylinder(sink, SCREW_D / 2.0, SCREW_HEAD / 2.0, seg), [0.0, 0.0, fl + 1.0 - sink])
        + translate(&cylinder(1.0, SCREW_HEAD / 2.0, SCREW_HEAD / 2.0, seg), [0.0, 0.0, fl + 1.0]);
    screw_spots(grid, piece, wall, p).into_iter().map(|[x, y]| translate(&hole, [x, y, -1.0])).collect()
}

/// Where the screws go: at the outermost magnet position of the piece's four outer corner cells,
/// or in their middle on a wall plate, clear of the hooks.
fn screw_spots(grid: &Grid, piece: &PlatePiece, wall: bool, p: &Profile) -> Vec<[f64; 2]> {
    let [fx, fy] = piece.first;
    let [cx, cy] = piece.cells;
    let m_pos = if wall { 0.0 } else { p.magnet.pos };
    let mut out = Vec::new();
    for (i, sx) in [(fx, -1.0), (fx + cx - 1, 1.0)] {
        for (j, sy) in [(fy, -1.0), (fy + cy - 1, 1.0)] {
            out.push([grid.line(0, i) + GRID / 2.0 + sx * m_pos, grid.line(1, j) + GRID / 2.0 + sy * m_pos]);
        }
    }
    out
}

/// Where the connectors sit along a cut: in the middle of every other cell of the piece's side,
/// `first` and `cells` counted along `axis`. Half a cell from either end of the cut, they keep
/// clear of the crossing cut and its connectors, of the screws in the corner cells and of the
/// magnets.
pub(crate) fn connector_spots(grid: &Grid, axis: usize, first: u32, cells: u32) -> Vec<f64> {
    (first..first + cells).step_by(2).map(|k| grid.line(axis, k) + GRID / 2.0).collect()
}

/// Dovetail tabs on the right and back cut lines of a piece (where a neighbour follows) and
/// the pockets for the neighbours' tabs on its left and front cut lines, and on a skeleton floor
/// the floor around those pockets, which the cross cut-out would otherwise take.
/// `sides`: a neighbour to the right, back, left, front.
fn connectors(grid: &Grid, piece: &PlatePiece, fl: f64, skeleton: bool, sides: [bool; 4]) -> [Vec<Solid>; 3] {
    let [right, back, left, front] = sides;
    let [fx, fy] = piece.first;
    let [cx, cy] = piece.cells;
    let (gap, big) = (CONNECTOR_GAP, 1.0);
    // A dovetail reaching from the line at `at` along `axis` into +axis, centred on `along`. A
    // tab stands on the build plate; a pocket (grown by the gap) is open below and keeps a skin
    // on top, so one piece is laid onto the other's tab and the dovetail holds them together.
    let tail = |axis: usize, at: f64, along: f64, grow: f64| -> Solid {
        let (n, hd, d) = (CONNECTOR_NECK / 2.0 + grow, CONNECTOR_HEAD / 2.0 + grow, CONNECTOR_DEPTH + grow);
        let pts: Vec<[f64; 2]> = if axis == 0 {
            vec![[at - big, along - n], [at + d, along - hd], [at + d, along + hd], [at - big, along + n]]
        } else {
            vec![[along - n, at - big], [along + n, at - big], [along + hd, at + d], [along - hd, at + d]]
        };
        dovetail_prism(&pts, fl, grow > 0.0)
    };
    // The floor a pocket needs around it, from the cut into the piece.
    let patch = |axis: usize, at: f64, along: f64| -> Solid {
        let (half, reach) = (CONNECTOR_HEAD / 2.0 + gap + CONNECTOR_WALL, CONNECTOR_DEPTH + gap + CONNECTOR_WALL);
        if axis == 0 {
            cuboid([reach, 2.0 * half, fl], [at, along - half, 0.0])
        } else {
            cuboid([2.0 * half, reach, fl], [along - half, at, 0.0])
        }
    };
    let [mut tabs, mut floor, mut pockets] = [Vec::new(), Vec::new(), Vec::new()];
    // A vertical cut (x fixed) carries its connectors at heights along y, a horizontal one at x.
    let ys = connector_spots(grid, 1, fy, cy);
    let xs = connector_spots(grid, 0, fx, cx);
    if right {
        tabs.extend(ys.iter().map(|&y| tail(0, grid.line(0, fx + cx), y, 0.0)));
    }
    if back {
        tabs.extend(xs.iter().map(|&x| tail(1, grid.line(1, fy + cy), x, 0.0)));
    }
    if left {
        let at = grid.line(0, fx);
        pockets.extend(ys.iter().map(|&y| tail(0, at, y, gap)));
        if skeleton {
            floor.extend(ys.iter().map(|&y| patch(0, at, y)));
        }
    }
    if front {
        let at = grid.line(1, fy);
        pockets.extend(xs.iter().map(|&x| tail(1, at, x, gap)));
        if skeleton {
            floor.extend(xs.iter().map(|&x| patch(1, at, x)));
        }
    }
    [tabs, floor, pockets]
}

/// A dovetail outline as a tab (from the build plate up to under the pocket's skin, with air)
/// or as a pocket (open below, a skin of `CONNECTOR_INSET` on top) in a floor `fl` thick.
pub(crate) fn dovetail_prism(pts: &[[f64; 2]], fl: f64, pocket: bool) -> Solid {
    if pocket {
        translate(&polygon_prism(pts, 0.0, fl - CONNECTOR_INSET + 1.0, 8), [0.0, 0.0, -1.0])
    } else {
        polygon_prism(pts, 0.0, fl - CONNECTOR_INSET - CONNECTOR_GAP, 8)
    }
}

/// A dovetail reaching from a plate's top edge at `top` down into it, centred on `x`, grown by
/// `grow`: the outline of a Skadis hook's tab (and, grown, its pocket).
pub(crate) fn top_tail(x: f64, top: f64, grow: f64) -> Vec<[f64; 2]> {
    let (n, hd, d) = (CONNECTOR_NECK / 2.0 + grow, CONNECTOR_HEAD / 2.0 + grow, CONNECTOR_DEPTH + grow);
    vec![[x - n, top + 1.0], [x - hd, top - d], [x + hd, top - d], [x + n, top + 1.0]]
}

/// The original script's plate: centred grid, solid margins, the given split.
pub fn pieces(spec: &PlateSpec, p: &Profile) -> Vec<Part> {
    let (grid, _) = Grid::new(spec.drawer, &spec.options(), &p.magnet).expect("plate spec fits its drawer");
    let claims = EdgeClaims { sides: Default::default(), corners: [false; 4] };
    let pieces = crate::plan::plate_pieces(&grid, grid.margins, &spec.split);
    let floor = if spec.magnets { PlateFloor::Magnets } else { PlateFloor::Open };
    pieces_of(&grid, &claims, &pieces, PlateBuild { floor, ..PlateBuild::default() }, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DrawerProject, DrawerSpec, Kind, PlateOptions};

    /// The ground plan of every connector a split makes, its pocket grown by the gap, as
    /// [x0, y0, x1, y1] in plate coordinates.
    fn footprints(grid: &Grid, pieces: &[PlatePiece]) -> Vec<[f64; 4]> {
        let (back, half, reach) = (1.0, CONNECTOR_HEAD / 2.0 + CONNECTOR_GAP, CONNECTOR_DEPTH + CONNECTOR_GAP);
        let has = |row: usize, col: usize| pieces.iter().any(|q| q.row == row && q.col == col);
        let mut out = Vec::new();
        for q in pieces {
            let [fx, fy] = q.first;
            let [cx, cy] = q.cells;
            if has(q.row, q.col + 1) {
                let x = grid.line(0, fx + cx);
                out.extend(connector_spots(grid, 1, fy, cy).iter().map(|&y| [x - back, y - half, x + reach, y + half]));
            }
            if has(q.row + 1, q.col) {
                let y = grid.line(1, fy + cy);
                out.extend(connector_spots(grid, 0, fx, cx).iter().map(|&x| [x - half, y - back, x + half, y + reach]));
            }
        }
        out
    }

    fn rect_gap(a: [f64; 4], b: [f64; 4]) -> f64 {
        let dx = (b[0] - a[2]).max(a[0] - b[2]).max(0.0);
        let dy = (b[1] - a[3]).max(a[1] - b[3]).max(0.0);
        dx.hypot(dy)
    }

    #[test]
    fn connectors_keep_clear_of_each_other_and_of_the_screws() {
        let p = Profile::default();
        // 7 x 5 cells, cut every way: uneven pieces, pieces one cell wide, one row only.
        let splits: [[Vec<u32>; 2]; 4] = [
            [vec![3, 2, 2], vec![2, 1, 2]],
            [vec![1; 7], vec![1; 5]],
            [vec![4, 3], vec![5]],
            [vec![2, 3, 2], vec![3, 2]],
        ];
        for wall in [false, true] {
            let kind = if wall { Kind::Wall } else { Kind::Surface };
            let project = DrawerProject {
                drawer: DrawerSpec { name: "Bank".into(), width: 294.0, depth: 210.0, height: 100.0 },
                options: PlateOptions { kind, ..PlateOptions::default() },
                ..DrawerProject::default()
            };
            let grid = crate::plan::plan(&project).unwrap().grid;
            for split in &splits {
                let pieces = crate::plan::plate_pieces(&grid, [0.0; 4], split);
                let feet = footprints(&grid, &pieces);
                for (i, &a) in feet.iter().enumerate() {
                    for &b in &feet[i + 1..] {
                        let gap = rect_gap(a, b);
                        assert!(gap > 10.0, "{split:?}: connectors {a:?} and {b:?} only {gap:.2} mm apart");
                    }
                    for q in &pieces {
                        for [x, y] in screw_spots(&grid, q, wall, &p) {
                            let gap = rect_gap(a, [x, y, x, y]) - SCREW_HEAD / 2.0;
                            assert!(gap > 3.0, "{split:?}: connector {a:?} only {gap:.2} mm from a screw at {x}, {y}");
                        }
                    }
                }
            }
        }
    }
}
