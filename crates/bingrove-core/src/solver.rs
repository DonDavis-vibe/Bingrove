//! Turning wishes into placements, deterministically: the same request always gives the same
//! layout. Tall things go to the back, low things to the front, the rest is filled with bins.

use crate::grid::Grid;
use crate::model::{BinSpec, Cover, HolderSpec, ItemKind, ItemRequest, Placement};
use crate::spec::{FLOOR_Z, GRID, HEADROOM, LIP_H, UNIT, WALL};
use crate::warn::Warning;

/// Bin heights for the back rows and for the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Heights {
    pub back: u32,
    pub front: u32,
    pub max: u32,
}

const DEFAULT_UNITS: u32 = 6;

/// Heights from the drawer's inner height (0 = unknown): the back as tall as fits, the front half of that.
pub fn heights(drawer_height: f64, grid: &Grid) -> (Heights, Option<Warning>) {
    if drawer_height <= 0.0 {
        let h = Heights { back: DEFAULT_UNITS, front: DEFAULT_UNITS, max: u32::MAX };
        return (h, Some(Warning::HeightUnknown { units: DEFAULT_UNITS }));
    }
    let max = ((drawer_height - HEADROOM - grid.floor - LIP_H) / UNIT).floor().max(1.0) as u32;
    (Heights { back: max, front: max.div_ceil(2).max(1).min(max), max }, None)
}

/// First row of the back band: the back third of the grid.
pub(crate) fn back_row(ny: u32) -> u32 {
    ny - (ny / 3).max(1).min(ny)
}

/// Cells a bin needs around an object of `size` (both orientations tried) and its height in units.
pub fn object_bin(size: [f64; 3], clearance: f64) -> ([u32; 2], u32) {
    let cells = |mm: f64| ((mm + 2.0 * WALL + clearance + 0.5) / GRID).ceil().max(1.0) as u32;
    let a = [cells(size[0]), cells(size[1])];
    let b = [cells(size[1]), cells(size[0])];
    let pick = if b[0] * b[1] < a[0] * a[1] { b } else { a };
    let units = ((FLOOR_Z + size[2] + clearance) / UNIT).ceil().max(2.0) as u32;
    (pick, units)
}

struct Want {
    name: String,
    cells: [u32; 2],
    units: Option<u32>,
    dividers: [u32; 2],
    holder: Option<HolderSpec>,
    order: usize,
}

fn wants(items: &[ItemRequest]) -> Vec<Want> {
    let mut out = Vec::new();
    for (order, item) in items.iter().enumerate() {
        for _ in 0..item.quantity.max(1) {
            let (cells, units, dividers, holder) = match &item.what {
                ItemKind::Bin { cells, units, dividers } => (*cells, *units, *dividers, None),
                ItemKind::Object { size, clearance } => {
                    let (cells, units) = object_bin(*size, *clearance);
                    (cells, Some(units), [1, 1], None)
                }
                ItemKind::Tape(t) => (t.cells(), Some(0), [1, 1], Some(HolderSpec::Tape(t.clone()))),
                ItemKind::Holder { holder, set, count } => {
                    let from_set = set
                        .as_deref()
                        .and_then(|id| crate::catalog::sets().into_iter().find(|s| s.id == id))
                        .map(|s| s.holder);
                    let Some(holder) = from_set.or_else(|| holder.clone()) else {
                        // Neither a holder nor a known set: a plain bin stands in for it.
                        out.push(Want {
                            name: item.name.clone(),
                            cells: [1, 1],
                            units: None,
                            dividers: [1, 1],
                            holder: None,
                            order,
                        });
                        continue;
                    };
                    let holder = match count {
                        Some(n) => holder.clone().with_cells(crate::catalog::cells_for(&holder, *n)),
                        None => holder,
                    };
                    (holder.cells(), Some(holder.units()), [1, 1], Some(holder))
                }
            };
            out.push(Want { name: item.name.clone(), cells, units, dividers, holder, order });
        }
    }
    // Tall first, then large, then in the order given.
    out.sort_by_key(|w| (std::cmp::Reverse(w.units.unwrap_or(0)), std::cmp::Reverse(w.cells[0] * w.cells[1]), w.order));
    out
}

/// Which cells are taken, `taken[y][x]`.
pub struct Occupancy {
    pub nx: u32,
    pub ny: u32,
    taken: Vec<Vec<bool>>,
}

impl Occupancy {
    pub fn new(nx: u32, ny: u32) -> Self {
        Occupancy { nx, ny, taken: vec![vec![false; nx as usize]; ny as usize] }
    }

    pub fn free(&self, x: u32, y: u32, [w, d]: [u32; 2]) -> bool {
        x + w <= self.nx
            && y + d <= self.ny
            && (y..y + d).all(|j| (x..x + w).all(|i| !self.taken[j as usize][i as usize]))
    }

    pub fn take(&mut self, x: u32, y: u32, [w, d]: [u32; 2]) {
        for j in y..y + d {
            for i in x..x + w {
                self.taken[j as usize][i as usize] = true;
            }
        }
    }
}

/// Places the requested items and fills the rest of the grid with bins.
pub fn auto_layout(grid: &Grid, items: &[ItemRequest], h: Heights) -> (Vec<Placement>, Vec<Warning>) {
    auto_layout_cover(grid, items, h, Cover::Exact)
}

/// [`auto_layout`] for a cover: with `Partial` the cells no item needs stay free.
pub fn auto_layout_cover(
    grid: &Grid,
    items: &[ItemRequest],
    h: Heights,
    cover: Cover,
) -> (Vec<Placement>, Vec<Warning>) {
    let [nx, ny] = grid.cells;
    let mut occ = Occupancy::new(nx, ny);
    let mut placements = Vec::new();
    let mut warnings = Vec::new();
    for w in wants(items) {
        let tall = w.units.is_some_and(|u| u > h.front) || w.holder.is_some();
        // A holder's pattern lies along its own axes, so only a plain bin may be turned.
        let ways = if w.holder.is_some() { vec![w.cells] } else { vec![w.cells, [w.cells[1], w.cells[0]]] };
        let spot = ways.into_iter().find_map(|cells| {
            let rows: Vec<u32> = if tall { (0..ny).rev().collect() } else { (0..ny).collect() };
            rows.into_iter().find_map(|y| (0..nx).find(|&x| occ.free(x, y, cells)).map(|x| (x, y, cells)))
        });
        let Some((x, y, cells)) = spot else {
            warnings.push(Warning::ItemUnplaced { item: w.name.clone(), cells: w.cells });
            continue;
        };
        occ.take(x, y, cells);
        let band = if y + cells[1] > back_row(ny) { h.back } else { h.front };
        let units = match w.units {
            Some(0) | None => band,
            Some(u) => u,
        };
        if w.holder.is_none() && units > h.max {
            warnings.push(Warning::ItemTooTall { item: w.name.clone(), units, max_units: h.max });
        }
        let dividers = if cells == w.cells { w.dividers } else { [w.dividers[1], w.dividers[0]] };
        let bin = BinSpec { cells, units, dividers, ..BinSpec::default() };
        let label = (!w.name.is_empty()).then(|| w.name.clone());
        placements.push(Placement { holder: w.holder, label, ..Placement::bin(x, y, bin) });
    }
    if cover == Cover::Exact {
        placements.extend(fill(&mut occ, h));
    }
    (placements, warnings)
}

/// Fills every free cell with bins of at most 2 x 2 cells that stay within their height band.
pub fn fill(occ: &mut Occupancy, h: Heights) -> Vec<Placement> {
    let split = back_row(occ.ny);
    let mut out = Vec::new();
    for y in 0..occ.ny {
        for x in 0..occ.nx {
            for cells in [[2, 2], [1, 2], [2, 1], [1, 1]] {
                let crosses = y < split && y + cells[1] > split;
                if !crosses && occ.free(x, y, cells) {
                    occ.take(x, y, cells);
                    let units = if y >= split { h.back } else { h.front };
                    out.push(Placement::bin(x, y, BinSpec { cells, units, ..BinSpec::default() }));
                    break;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::validate;
    use crate::model::{PlateOptions, TapeSpec};
    use crate::profile::MagnetSpec;

    fn grid() -> Grid {
        Grid::new([250.0, 305.0], &PlateOptions::default(), &MagnetSpec::default()).unwrap().0
    }

    #[test]
    fn heights_like_the_small_drawer() {
        let (h, w) = heights(100.0, &grid());
        assert_eq!((h.back, h.front), (12, 6));
        assert!(w.is_none());
        assert!(heights(0.0, &grid()).1.is_some());
    }

    #[test]
    fn objects_get_bins_around_them() {
        // A 60 x 35 x 25 bit set: 2 x 1 cells, (5.95 + 25 + 2) / 7 -> 5 units.
        assert_eq!(object_bin([60.0, 35.0, 25.0], 2.0), ([2, 1], 5));
        // 40 mm plus walls and air no longer fit the 39.1 mm inside of one cell.
        assert_eq!(object_bin([60.0, 40.0, 25.0], 2.0).0, [2, 2]);
        // Longer than wide but the other way round would need more cells.
        assert_eq!(object_bin([30.0, 100.0, 10.0], 2.0), ([1, 3], 3));
    }

    #[test]
    fn layout_covers_the_grid_and_is_deterministic() {
        let g = grid();
        let (h, _) = heights(100.0, &g);
        let items = vec![
            ItemRequest {
                name: "Kabel".into(),
                quantity: 2,
                what: ItemKind::Bin { cells: [2, 2], units: Some(12), dividers: [1, 1] },
            },
            ItemRequest {
                name: "Stifte".into(),
                quantity: 1,
                what: ItemKind::Object { size: [20.0, 180.0, 20.0], clearance: 2.0 },
            },
            ItemRequest {
                name: "Bits".into(),
                quantity: 1,
                what: ItemKind::Object { size: [60.0, 35.0, 25.0], clearance: 2.0 },
            },
        ];
        let (a, warnings) = auto_layout(&g, &items, h);
        assert!(warnings.is_empty(), "{warnings:?}");
        validate(5, 7, &a).unwrap();
        let (b, _) = auto_layout(&g, &items, h);
        assert_eq!(a, b);
        // Tall cable bins at the back, the pen tray at the front.
        let kabel: Vec<_> = a.iter().filter(|p| p.label.as_deref() == Some("Kabel")).collect();
        assert!(kabel.iter().all(|p| p.y + p.bin.cells[1] == 7 && p.bin.units == 12));
        let stifte = a.iter().find(|p| p.label.as_deref() == Some("Stifte")).unwrap();
        assert_eq!((stifte.y, stifte.bin.cells), (0, [1, 5]));
    }

    #[test]
    fn unplaceable_and_too_tall_items_are_reported() {
        let g = grid();
        let (h, _) = heights(60.0, &g);
        let items = vec![
            ItemRequest {
                name: "Riese".into(),
                quantity: 1,
                what: ItemKind::Bin { cells: [6, 6], units: None, dividers: [1, 1] },
            },
            ItemRequest {
                name: "Flasche".into(),
                quantity: 1,
                what: ItemKind::Object { size: [30.0, 30.0, 150.0], clearance: 2.0 },
            },
            ItemRequest { name: "Tape".into(), quantity: 1, what: ItemKind::Tape(TapeSpec::default()) },
        ];
        let (p, w) = auto_layout(&g, &items, h);
        assert!(w.iter().any(|w| matches!(w, Warning::ItemUnplaced { item, .. } if item == "Riese")));
        assert!(w.iter().any(|w| matches!(w, Warning::ItemTooTall { item, .. } if item == "Flasche")));
        assert!(p.iter().any(|p| p.holder.is_some() && p.cells() == [3, 3]));
        validate(5, 7, &p).unwrap();
    }
}
