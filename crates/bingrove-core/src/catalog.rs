//! What holders there are and how big they come out, for the solver, the panel and the AI.

use serde::Serialize;

use crate::model::{BIT_ACROSS_FLATS, BIT_CLEARANCE, HolderSpec, HolesSpec, RoundItem, TroughSpec};
use crate::spec::{GRID, WALL, bin_outer};

/// Wall between two holes of a pattern.
pub const HOLE_WALL: f64 = 1.6;
/// Air between the pattern and the bin wall.
pub const PATTERN_MARGIN: f64 = 0.8;

/// How many of a pitch fit into the inside of a bin of `cells` along one axis.
fn fit(cells: u32, pitch: f64) -> u32 {
    let inner = bin_outer(cells) - 2.0 * WALL - 2.0 * PATTERN_MARGIN;
    ((inner + HOLE_WALL) / pitch).floor().max(0.0) as u32
}

/// Hole pitch of a round item.
pub fn hole_pitch(item: &RoundItem, clearance: f64) -> f64 {
    item.size().0 + clearance + HOLE_WALL
}

pub fn bit_pitch() -> f64 {
    // Across corners of the hex plus the wall.
    (BIT_ACROSS_FLATS + BIT_CLEARANCE) / 3f64.sqrt() * 2.0 + HOLE_WALL
}

/// Holes across and down for a holder of `cells`.
pub fn holes_grid(spec: &HolesSpec) -> [u32; 2] {
    let pitch = hole_pitch(&spec.item, spec.clearance);
    spec.cells.map(|c| fit(c, pitch))
}

pub fn bits_grid(cells: [u32; 2]) -> [u32; 2] {
    cells.map(|c| fit(c, bit_pitch()))
}

/// Troughs across the bin (along y) for a trough holder.
pub fn trough_count(spec: &TroughSpec) -> u32 {
    let pitch = spec.handle.unwrap_or(spec.diameter) + 1.0 + HOLE_WALL;
    fit(spec.cells[1], pitch)
}

/// Capacity of a holder: holes, bits, troughs or tape stacks.
pub fn capacity(h: &HolderSpec) -> u32 {
    match h {
        HolderSpec::Tape(t) => t.stacks[0] * t.stacks[1],
        HolderSpec::Holes(s) => holes_grid(s).iter().product(),
        HolderSpec::Bits(b) => bits_grid(b.cells).iter().product(),
        HolderSpec::Troughs(t) => trough_count(t),
    }
}

/// The smallest footprint (by area, then the squarer one) that holds `count` things.
pub fn cells_for(h: &HolderSpec, count: u32) -> [u32; 2] {
    if matches!(h, HolderSpec::Tape(_)) {
        return h.cells();
    }
    let mut best: Option<[u32; 2]> = None;
    for area in 1..=36u32 {
        for w in 1..=6u32 {
            if area % w != 0 || area / w > 6 {
                continue;
            }
            let cells = [w, area / w];
            // Troughs want length along x: keep at least the given x cells.
            if let HolderSpec::Troughs(t) = h
                && cells[0] < t.cells[0]
            {
                continue;
            }
            if capacity(&h.clone().with_cells(cells)) >= count
                && best.is_none_or(|b| (cells[0] as i64 - cells[1] as i64).abs() < (b[0] as i64 - b[1] as i64).abs())
            {
                best = Some(cells);
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.unwrap_or([6, 6])
}

/// Cells along x a lying thing of `length` wants (about 60 % of it supported).
pub fn trough_cells_x(length: f64) -> u32 {
    ((length * 0.6 + 2.0 * WALL) / GRID).ceil().max(1.0) as u32
}

/// One entry of the catalogue, for the AI and the panel.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub holder: HolderSpec,
}

pub fn catalog() -> Vec<CatalogEntry> {
    let holes = |item: RoundItem| HolderSpec::Holes(HolesSpec { item, ..HolesSpec::default() });
    vec![
        CatalogEntry { id: "tape", label: "Tape-Rollen", holder: HolderSpec::Tape(Default::default()) },
        CatalogEntry { id: "aa", label: "AA-Zellen", holder: holes(RoundItem::Aa) },
        CatalogEntry { id: "aaa", label: "AAA-Zellen", holder: holes(RoundItem::Aaa) },
        CatalogEntry { id: "c", label: "C-Zellen", holder: holes(RoundItem::C) },
        CatalogEntry { id: "d", label: "D-Zellen", holder: holes(RoundItem::D) },
        CatalogEntry { id: "18650", label: "18650-Akkus", holder: holes(RoundItem::Cell18650) },
        CatalogEntry { id: "21700", label: "21700-Akkus", holder: holes(RoundItem::Cell21700) },
        CatalogEntry { id: "bits", label: "1/4\"-Bits", holder: HolderSpec::Bits(Default::default()) },
        CatalogEntry { id: "pens", label: "Stifte (stehend)", holder: holes(RoundItem::Custom { d: 12.0, h: 140.0 }) },
        CatalogEntry {
            id: "pens_lying",
            label: "Stifte (liegend)",
            holder: HolderSpec::Troughs(TroughSpec { diameter: 12.0, length: 140.0, ..TroughSpec::default() }),
        },
        CatalogEntry {
            id: "screwdrivers",
            label: "Schraubendreher",
            holder: HolderSpec::Troughs(TroughSpec {
                diameter: 8.0,
                length: 200.0,
                handle: Some(30.0),
                cells: [4, 1],
                ..TroughSpec::default()
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacities_and_sizes() {
        // 39.1 mm inside a 1x1 bin minus margins: two AA holes (pitch 16.7) fit each way.
        let aa = HolderSpec::Holes(HolesSpec { item: RoundItem::Aa, ..HolesSpec::default() });
        assert_eq!(capacity(&aa), 4);
        assert_eq!(cells_for(&aa, 4), [1, 1]);
        // 1x2 holds 2 x 4 = 8, so twelve need three cells (2 x 6).
        let twelve = cells_for(&aa, 12);
        assert!(capacity(&aa.clone().with_cells(twelve)) >= 12 && twelve[0] * twelve[1] == 3, "{twelve:?}");
        let bits = HolderSpec::Bits(Default::default());
        assert_eq!(capacity(&bits), 16, "4 x 4 bits at 9.1 mm pitch in a 1x1 bin");
        let sd = catalog().into_iter().find(|c| c.id == "screwdrivers").unwrap().holder;
        assert_eq!(capacity(&sd), 1);
        assert_eq!(cells_for(&sd, 3)[0], 4, "troughs keep their length");
        assert!(sd.units() >= 5);
        assert_eq!(aa.units(), 6, "half an AA cell shows above the holder");
    }
}
