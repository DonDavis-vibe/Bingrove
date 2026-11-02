//! A complete plan of a drawer without any geometry: grid, margins, plate pieces, placements
//! with their edge extents, the parts list and warnings. Fast enough for every keystroke and
//! every AI round trip.

use serde::{Deserialize, Serialize};

use crate::grid::{Grid, PlanError, Zone};
use crate::layout::{Repair, repair_cover, validate_cover};
use crate::model::{BinSpec, Cover, DrawerProject, EdgeMode, HolderSpec, Kind, Mount, Placement, PlateFloor, Side};
use crate::solver::{Heights, auto_layout_cover, heights};
use crate::spec::{BIN_GAP, CONNECTOR_DEPTH, EDGE_GAP, GRID, LIP_H, bin_outer, body_height};
use crate::split::{extents, split_plate};
use crate::warn::Warning;

/// One printable piece of the plate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatePiece {
    /// Row from the front, column from the left.
    pub row: usize,
    pub col: usize,
    pub cells: [u32; 2],
    /// First cell of the piece.
    pub first: [u32; 2],
    /// The cut box in plate coordinates (x0, y0, x1, y1).
    pub rect: [f64; 4],
    /// Size of the piece as printed.
    pub size: [f64; 2],
}

/// Which edge-zone segments are taken over by edge bins (the plate is cut back there in
/// ToFloor mode). Left/right are indexed by row, front/back by column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeClaims {
    pub sides: [Vec<bool>; 4],
    /// Front-left, front-right, back-left, back-right.
    pub corners: [bool; 4],
}

impl EdgeClaims {
    pub fn corner(left: bool, front: bool) -> usize {
        match (left, front) {
            (true, true) => 0,
            (false, true) => 1,
            (true, false) => 2,
            (false, false) => 3,
        }
    }
}

/// A distinct part, enough to make its geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PartSpec {
    /// Index into [`Plan::pieces`].
    Plate {
        piece: usize,
    },
    Bin {
        spec: BinSpec,
        /// How far the bin reaches into the margin per side (left, right, front, back).
        extents: [f64; 4],
        edge_mode: EdgeMode,
    },
    Holder(HolderSpec),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BomEntry {
    pub name: String,
    pub part: PartSpec,
    pub quantity: u32,
    /// Outer size x, y, z.
    pub size: [f64; 3],
}

/// A placement with what it became.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedPart {
    #[serde(flatten)]
    pub placement: Placement,
    pub extents: [f64; 4],
    /// Index into [`Plan::parts`].
    pub part: usize,
    /// The part is the BOM's one turned by 180°.
    pub turned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub name: String,
    pub grid: Grid,
    pub split: [Vec<u32>; 2],
    pub pieces: Vec<PlatePiece>,
    pub claims: EdgeClaims,
    pub heights: Heights,
    pub placements: Vec<PlacedPart>,
    pub parts: Vec<BomEntry>,
    pub magnets: bool,
    pub floor: PlateFloor,
    /// Screws and connectors of the plate (none for an open frame).
    pub mount: Mount,
    pub kind: Kind,
    pub cover: Cover,
    pub warnings: Vec<Warning>,
    pub repairs: Vec<Repair>,
}

fn quantise(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Sides a placement touches, as (side, touches).
fn touches(p: &Placement, cells: [u32; 2]) -> [bool; 4] {
    let [w, d] = p.cells();
    [p.x == 0, p.x + w == cells[0], p.y == 0, p.y + d == cells[1]]
}

/// How far a placement reaches into each margin.
pub fn edge_extents(grid: &Grid, p: &Placement) -> [f64; 4] {
    let t = touches(p, grid.cells);
    Side::ALL.map(|s| {
        let i = s as usize;
        if t[i] && grid.zones[i] == Zone::Edge && !p.edges.off[i] && p.holder.is_none() {
            quantise(grid.margins[i] + BIN_GAP - EDGE_GAP)
        } else {
            0.0
        }
    })
}

fn claims(grid: &Grid, placed: &[(Placement, [f64; 4])]) -> EdgeClaims {
    let [nx, ny] = grid.cells;
    let edge = |s: Side| grid.zone(s) == Zone::Edge;
    if placed.is_empty() {
        // A plate on its own: assume edge bins everywhere.
        let sides = Side::ALL.map(|s| vec![edge(s); if (s as usize) < 2 { ny } else { nx } as usize]);
        let corners = [
            (Side::Left, Side::Front),
            (Side::Right, Side::Front),
            (Side::Left, Side::Back),
            (Side::Right, Side::Back),
        ]
        .map(|(a, b)| edge(a) && edge(b));
        return EdgeClaims { sides, corners };
    }
    let mut c = EdgeClaims {
        sides: [vec![false; ny as usize], vec![false; ny as usize], vec![false; nx as usize], vec![false; nx as usize]],
        corners: [false; 4],
    };
    for (p, e) in placed {
        let [w, d] = p.cells();
        for s in Side::ALL {
            if e[s as usize] > 0.0 {
                let range = if (s as usize) < 2 { p.y..p.y + d } else { p.x..p.x + w };
                for k in range {
                    c.sides[s as usize][k as usize] = true;
                }
            }
        }
        for (l, f) in [(true, true), (false, true), (true, false), (false, false)] {
            let (sx, sy) = (if l { Side::Left } else { Side::Right }, if f { Side::Front } else { Side::Back });
            if e[sx as usize] > 0.0 && e[sy as usize] > 0.0 {
                c.corners[EdgeClaims::corner(l, f)] = true;
            }
        }
    }
    c
}

/// Outer size of a bin with extents.
pub fn bin_size(spec: &BinSpec, e: [f64; 4]) -> [f64; 3] {
    [
        bin_outer(spec.cells[0]) + e[0] + e[1],
        bin_outer(spec.cells[1]) + e[2] + e[3],
        body_height(spec.units) + if spec.lip { LIP_H } else { 0.0 },
    ]
}

/// The canonical form of a bin part: of it and its 180° turn, the one reaching into the left
/// (then front) margin (a bin with a label or scoop cannot be turned).
fn canonical(spec: &BinSpec, e: [f64; 4]) -> ([f64; 4], bool) {
    let turned = [e[1], e[0], e[3], e[2]];
    let key = |v: &[f64; 4]| v.map(|x| (x * 100.0).round() as i64);
    if spec.label_tab || spec.scoop > 0 || key(&e) >= key(&turned) { (e, false) } else { (turned, true) }
}

fn part_name(part: &PartSpec, pieces: &[PlatePiece], floor: PlateFloor, mount: Mount) -> String {
    match part {
        PartSpec::Plate { piece } => {
            let p = &pieces[*piece];
            crate::names::baseplate(p.row, p.col, p.cells, floor, mount)
        }
        PartSpec::Bin { spec, extents, edge_mode } => crate::names::edge_bin(spec, *extents, *edge_mode),
        PartSpec::Holder(h) => crate::names::holder(h),
    }
}

pub fn plan(project: &DrawerProject) -> Result<Plan, PlanError> {
    let d = &project.drawer;
    let o = &project.options;
    let (grid, mut warnings) = Grid::new([d.width, d.depth], o, &project.profile.magnet)?;
    let [nx, ny] = grid.cells;
    let (h, hw) = heights(d.height, &grid);
    warnings.extend(hw);

    let mut repairs = Vec::new();
    // A surface stays open where nothing is put; a drawer is covered unless asked otherwise.
    let cover = if o.kind == Kind::Surface { Cover::Partial } else { o.cover };
    let placements = if project.placements.is_empty() && (cover == Cover::Exact || !project.items.is_empty()) {
        let (p, w) = auto_layout_cover(&grid, &project.items, h, cover);
        warnings.extend(w);
        p
    } else if validate_cover(nx, ny, &project.placements, cover).is_ok() {
        project.placements.clone()
    } else {
        let (p, r) = repair_cover(nx, ny, project.placements.clone(), h, cover);
        repairs = r;
        p
    };
    validate_cover(nx, ny, &placements, cover)?;

    let placed: Vec<(Placement, [f64; 4])> = placements
        .into_iter()
        .map(|p| {
            let e = edge_extents(&grid, &p);
            (p, e)
        })
        .collect();
    let claims = claims(&grid, &placed);

    let split = match &o.split {
        Some(s) => {
            for (axis, s) in s.iter().enumerate() {
                if s.iter().sum::<u32>() != grid.cells[axis] {
                    return Err(PlanError::BadSplit(s.clone(), grid.cells[axis]));
                }
            }
            s.clone()
        }
        None => split_plate(&grid, reach(&grid, &claims), &project.bed)
            .unwrap_or_else(|| [vec![1; nx as usize], vec![1; ny as usize]]),
    };
    let mut pieces = plate_pieces(&grid, reach(&grid, &claims), &split);
    let magnets = o.floor == PlateFloor::Magnets;
    // Screws and connectors need a floor; the connectors' tabs make a piece longer as printed.
    let mount = if o.floor == PlateFloor::Open { Mount::default() } else { o.mount };
    if mount.connectors {
        for piece in &mut pieces {
            if piece.col + 1 < split[0].len() {
                piece.size[0] += CONNECTOR_DEPTH;
            }
            if piece.row + 1 < split[1].len() {
                piece.size[1] += CONNECTOR_DEPTH;
            }
        }
    }

    let mut parts: Vec<BomEntry> = Vec::new();
    for (i, piece) in pieces.iter().enumerate() {
        let part = PartSpec::Plate { piece: i };
        let name = part_name(&part, &pieces, o.floor, mount);
        parts.push(BomEntry { name, part, quantity: 1, size: [piece.size[0], piece.size[1], grid.height()] });
    }
    let mut out = Vec::with_capacity(placed.len());
    for (p, e) in placed {
        let (part, size, turned) = match &p.holder {
            Some(h) => (PartSpec::Holder(h.clone()), h.size(), false),
            None => {
                let (ce, turned) = canonical(&p.bin, e);
                let mode = if ce.iter().any(|&x| x > 0.0) { grid.edge_mode } else { EdgeMode::default() };
                (PartSpec::Bin { spec: p.bin.clone(), extents: ce, edge_mode: mode }, bin_size(&p.bin, ce), turned)
            }
        };
        let index = match parts.iter().position(|b| b.part == part) {
            Some(i) => {
                parts[i].quantity += 1;
                i
            }
            None => {
                let name = part_name(&part, &pieces, o.floor, mount);
                parts.push(BomEntry { name, part, quantity: 1, size });
                parts.len() - 1
            }
        };
        out.push(PlacedPart { placement: p, extents: e, part: index, turned });
    }

    for b in &parts {
        if !project.bed.fits(b.size[0], b.size[1]) {
            warnings.push(Warning::PartTooLarge { part: b.name.clone(), size: [b.size[0], b.size[1]] });
        }
    }
    if grid.edge_mode == EdgeMode::ToFloor && grid.floor == 0.0 {
        let sides: Vec<Side> = Side::ALL
            .into_iter()
            .filter(|&s| grid.cut_back(s) && claims.sides[s as usize].iter().any(|&c| c))
            .collect();
        if !sides.is_empty() {
            warnings.push(Warning::PlateHeldByBins { sides });
        }
    }

    Ok(Plan {
        name: d.name.clone(),
        grid,
        split,
        pieces,
        claims,
        heights: h,
        placements: out,
        parts,
        magnets,
        floor: o.floor,
        mount,
        kind: o.kind,
        cover,
        warnings,
        repairs,
    })
}

/// How far the plate reaches past the outer grid line per side: just the rim where edge bins
/// take over the whole side, the margin otherwise.
pub fn reach(grid: &Grid, claims: &EdgeClaims) -> [f64; 4] {
    Side::ALL.map(|s| {
        let all = claims.sides[s as usize].iter().all(|&c| c);
        if all { grid.plate_reach(s) } else { grid.margin(s) }
    })
}

/// The pieces of a split: cut boxes from the plate outline to the grid lines between pieces.
pub fn plate_pieces(grid: &Grid, reach: [f64; 4], split: &[Vec<u32>; 2]) -> Vec<PlatePiece> {
    let ex = extents(reach, 0, &split[0]);
    let ey = extents(reach, 1, &split[1]);
    // Cut boxes run from the plate outline to the grid lines between pieces.
    let cuts = |axis: usize| -> Vec<f64> {
        let mut v = vec![0.0];
        let mut acc = 0;
        for &n in &split[axis][..split[axis].len() - 1] {
            acc += n;
            v.push(grid.line(axis, acc));
        }
        v.push(grid.plate[axis]);
        v
    };
    let (xs, ys) = (cuts(0), cuts(1));
    let mut out = Vec::new();
    let mut fy = 0;
    for (row, &cy) in split[1].iter().enumerate() {
        let mut fx = 0;
        for (col, &cx) in split[0].iter().enumerate() {
            out.push(PlatePiece {
                row,
                col,
                cells: [cx, cy],
                first: [fx, fy],
                rect: [xs[col], ys[row], xs[col + 1], ys[row + 1]],
                size: [ex[col], ey[row]],
            });
            fx += cx;
        }
        fy += cy;
    }
    out
}

impl Plan {
    /// Where a placed part's minimum corner goes, in plate coordinates, with the bins standing
    /// on the plate floor.
    pub fn position(&self, p: &PlacedPart) -> [f64; 3] {
        let g = &self.grid;
        let x = g.line(0, p.placement.x) + BIN_GAP - p.extents[0];
        let y = g.line(1, p.placement.y) + BIN_GAP - p.extents[2];
        [x, y, g.floor]
    }

    /// Total number of printed parts.
    pub fn part_count(&self) -> u32 {
        self.parts.iter().map(|b| b.quantity).sum()
    }

    /// Area of the drawer usable from above by bins, in percent of the plate.
    pub fn usable_area(&self) -> f64 {
        let g = &self.grid;
        let used: f64 = self
            .placements
            .iter()
            .map(|p| {
                let [w, d] = p.placement.cells();
                (w as f64 * GRID + p.extents[0] + p.extents[1]) * (d as f64 * GRID + p.extents[2] + p.extents[3])
            })
            .sum();
        100.0 * used / (g.plate[0] * g.plate[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DrawerSpec, PlateOptions};
    use crate::presets::Preset;

    fn small(options: PlateOptions) -> DrawerProject {
        let p = Preset::get("250x305").unwrap();
        DrawerProject {
            drawer: DrawerSpec { name: "Kleinkram".into(), width: 250.0, depth: 305.0, height: 100.0 },
            options,
            placements: p.placements,
            ..DrawerProject::default()
        }
    }

    #[test]
    fn a_surface_stays_open_where_nothing_stands() {
        use crate::model::{ItemKind, ItemRequest, Kind};
        let options = PlateOptions { kind: Kind::Surface, ..PlateOptions::default() };
        let empty = DrawerProject {
            drawer: DrawerSpec { name: "Bank".into(), width: 600.0, depth: 300.0, height: 100.0 },
            options: options.clone(),
            ..DrawerProject::default()
        };
        let p = plan(&empty).unwrap();
        assert_eq!(p.grid.cells, [14, 7]);
        assert!(p.placements.is_empty(), "no bins are filled in");
        assert!(p.parts.iter().all(|b| matches!(b.part, PartSpec::Plate { .. })));
        assert_eq!(p.usable_area(), 0.0);
        assert!(crate::text::ascii_plan(&p).contains(" .  .  . "));
        // One item is placed, the rest of the grid stays free.
        let one = DrawerProject {
            items: vec![ItemRequest {
                name: "Bits".into(),
                quantity: 1,
                what: ItemKind::Bin { cells: [2, 2], units: Some(6), dividers: [1, 1] },
            }],
            ..empty.clone()
        };
        let p = plan(&one).unwrap();
        assert_eq!(p.placements.len(), 1);
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    }

    #[test]
    fn edge_bins_of_the_small_drawer() {
        let plan = plan(&small(PlateOptions::default())).unwrap();
        assert_eq!(plan.split, [vec![5], vec![4, 3]]);
        // The long pen tray on the left reaches 19.25 mm into the margin, 0.5 mm short of the plate outline.
        let tray = plan.placements.iter().find(|p| p.placement.bin.cells == [1, 5]).unwrap();
        assert_eq!(tray.extents, [19.25, 0.0, 0.0, 0.0]);
        assert_eq!(plan.parts[tray.part].name, "randbin_1x5_6u_L19.25.stl");
        assert_eq!(plan.parts[tray.part].size[0], 41.5 + 19.25);
        // The two 1x1 bins at the front right reach into the right margin and are one part.
        let right_small: Vec<_> =
            plan.placements.iter().filter(|p| p.placement.bin.cells == [1, 1] && p.extents[1] > 0.0).collect();
        assert_eq!(right_small.len(), 2);
        assert_eq!(right_small[0].part, right_small[1].part);
        assert_eq!(plan.parts[right_small[0].part].quantity, 2);
        assert!(plan.claims.sides[0].iter().all(|&c| c) && plan.claims.sides[1].iter().all(|&c| c));
        assert_eq!(
            plan.warnings,
            [
                Warning::AlignForEdgeZone { axis: 1, centred: 5.0, one_side: 10.0 },
                Warning::PlateHeldByBins { sides: vec![Side::Left, Side::Right] },
            ]
        );
        assert!(plan.usable_area() > 96.0, "{}", plan.usable_area());
    }

    #[test]
    fn turned_edge_bins_share_a_part() {
        // Three cells with 15 mm edge zones left and right: the outer bins are one part, turned.
        let project = DrawerProject {
            drawer: DrawerSpec { name: "schmal".into(), width: 157.0, depth: 45.0, height: 0.0 },
            placements: (0..3).map(|x| Placement::bin(x, 0, BinSpec::default())).collect(),
            ..DrawerProject::default()
        };
        let plan = plan(&project).unwrap();
        let [l, m, r] = [0, 1, 2].map(|i| &plan.placements[i]);
        assert_eq!(l.extents, [14.75, 0.0, 0.0, 0.0]);
        assert_eq!(r.extents, [0.0, 14.75, 0.0, 0.0]);
        assert_eq!(l.part, r.part);
        assert!(!l.turned && r.turned);
        assert_ne!(m.part, l.part);
        assert_eq!(plan.parts[l.part].name, "randbin_1x1_6u_L14.75.stl");
        assert_eq!(plan.parts[l.part].quantity, 2);
    }

    #[test]
    fn solid_margins_reproduce_the_python_parts() {
        let o = PlateOptions { edge_threshold: f64::INFINITY, ..PlateOptions::default() };
        let plan = plan(&small(o)).unwrap();
        assert_eq!(plan.split, [vec![3, 2], vec![4, 3]]);
        let names: Vec<_> = plan.parts.iter().map(|b| (b.name.as_str(), b.quantity)).collect();
        assert_eq!(
            names,
            [
                ("baseplate_1A_3x4.stl", 1),
                ("baseplate_1B_2x4.stl", 1),
                ("baseplate_2A_3x3.stl", 1),
                ("baseplate_2B_2x3.stl", 1),
                ("bin_2x2_12u.stl", 2),
                ("bin_1x2_12u.stl", 1),
                ("bin_1x5_6u.stl", 1),
                ("bin_2x3_6u.stl", 1),
                ("bin_2x2_6u.stl", 1),
                ("bin_2x1_6u_2x1fach.stl", 1),
                ("bin_1x1_6u.stl", 8),
            ]
        );
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
        assert!(plan.placements.iter().all(|p| p.extents == [0.0; 4]));
    }

    #[test]
    fn positions_put_edge_bins_against_the_outline() {
        let plan = plan(&small(PlateOptions::default())).unwrap();
        let tray = plan.placements.iter().find(|p| p.placement.bin.cells == [1, 5]).unwrap();
        let pos = plan.position(tray);
        assert!((pos[0] - EDGE_GAP).abs() < 1e-9, "{pos:?}");
    }
}
