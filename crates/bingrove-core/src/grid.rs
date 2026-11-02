//! Where the grid sits in the drawer and what happens to the leftover margins.
//!
//! Plate coordinates: x to the right, y to the back, (0, 0) at the plate's front-left corner.

use serde::{Deserialize, Serialize};

use crate::model::{Align, EdgeMode, Kind, PlateFloor, PlateOptions, Side};
use crate::profile::MagnetSpec;
use crate::spec::{BP_H, GRID};
use crate::warn::Warning;

/// What a margin is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    /// Part of the plate, keeps it tight in the drawer.
    Solid,
    /// Bins touching this side reach into it.
    Edge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub cells: [u32; 2],
    /// Outline of the plate.
    pub plate: [f64; 2],
    /// Front-left corner of the grid in plate coordinates.
    pub origin: [f64; 2],
    /// Margin per side (left, right, front, back).
    pub margins: [f64; 4],
    pub zones: [Zone; 4],
    pub edge_mode: EdgeMode,
    pub rim: f64,
    /// Thickness of the plate floor (0 for an open frame).
    pub floor: f64,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlanError {
    #[error("the drawer ({0:.1} x {1:.1} mm) is too small for one grid cell")]
    TooSmall(f64, f64),
    #[error("{0} x {1} cells do not fit the drawer (at most {2} x {3})")]
    TooManyCells(u32, u32, u32, u32),
    #[error("the split {0:?} does not add up to {1} cells")]
    BadSplit(Vec<u32>, u32),
    #[error("{0}")]
    Layout(#[from] crate::layout::LayoutError),
}

impl Grid {
    pub fn new(drawer: [f64; 2], o: &PlateOptions, magnet: &MagnetSpec) -> Result<(Grid, Vec<Warning>), PlanError> {
        let floor = match o.floor {
            PlateFloor::Open => 0.0,
            PlateFloor::Magnets => magnet.h + magnet.floor,
        };
        if o.kind == Kind::Surface {
            // The plate is the grid: as many cells as the size holds (or as asked), no margins.
            let fit = drawer.map(|d| (d / GRID + 1e-9).floor().max(0.0) as u32);
            let cells = o.cells.unwrap_or(fit);
            if cells[0] == 0 || cells[1] == 0 {
                return Err(PlanError::TooSmall(drawer[0], drawer[1]));
            }
            let plate = cells.map(|c| c as f64 * GRID);
            let grid = Grid {
                cells,
                plate,
                origin: [0.0; 2],
                margins: [0.0; 4],
                zones: [Zone::Solid; 4],
                edge_mode: EdgeMode::None,
                rim: o.rim,
                floor,
            };
            return Ok((grid, Vec::new()));
        }
        let plate = drawer.map(|d| d - o.clearance);
        let fit = plate.map(|p| (p / GRID + 1e-9).floor().max(0.0) as u32);
        if fit[0] == 0 || fit[1] == 0 {
            return Err(PlanError::TooSmall(drawer[0], drawer[1]));
        }
        let cells = o.cells.unwrap_or(fit);
        if cells[0] > fit[0] || cells[1] > fit[1] || cells[0] == 0 || cells[1] == 0 {
            return Err(PlanError::TooManyCells(cells[0], cells[1], fit[0], fit[1]));
        }
        let mut margins = [0.0; 4];
        let mut origin = [0.0; 2];
        let mut warnings = Vec::new();
        for axis in 0..2 {
            let left = plate[axis] - cells[axis] as f64 * GRID;
            let (lo, hi) = match o.align[axis] {
                Align::Start => (0.0, left),
                Align::Center => (left / 2.0, left / 2.0),
                Align::End => (left, 0.0),
            };
            margins[2 * axis] = lo;
            margins[2 * axis + 1] = hi;
            origin[axis] = lo;
            let wants_edges = o.edge_mode != EdgeMode::None;
            if wants_edges
                && o.align[axis] == Align::Center
                && left / 2.0 < o.edge_threshold
                && left >= o.edge_threshold
            {
                warnings.push(Warning::AlignForEdgeZone { axis, centred: left / 2.0, one_side: left });
            }
        }
        let zones = match o.edge_mode {
            EdgeMode::None => [Zone::Solid; 4],
            _ => margins.map(|m| if m >= o.edge_threshold { Zone::Edge } else { Zone::Solid }),
        };
        let grid = Grid { cells, plate, origin, margins, zones, edge_mode: o.edge_mode, rim: o.rim, floor };
        if grid.zones.contains(&Zone::Edge) && grid.edge_mode == EdgeMode::Resting {
            warnings.push(Warning::SupportsNeeded);
        }
        Ok((grid, warnings))
    }

    /// Plate height.
    pub fn height(&self) -> f64 {
        BP_H + self.floor
    }

    /// Grid line `i` along `axis` in plate coordinates.
    pub fn line(&self, axis: usize, i: u32) -> f64 {
        self.origin[axis] + i as f64 * GRID
    }

    pub fn zone(&self, side: Side) -> Zone {
        self.zones[side as usize]
    }

    pub fn margin(&self, side: Side) -> f64 {
        self.margins[side as usize]
    }

    /// Whether the plate is cut back to the grid on this side (ToFloor edge zone).
    pub fn cut_back(&self, side: Side) -> bool {
        self.zone(side) == Zone::Edge && self.edge_mode == EdgeMode::ToFloor
    }

    /// How far the plate reaches past the outer grid line on a side (for piece sizes):
    /// the whole margin, or just the rim where ToFloor edge bins take over. A floor plate keeps
    /// its floor strip out to the wall.
    pub fn plate_reach(&self, side: Side) -> f64 {
        if self.cut_back(side) && self.floor == 0.0 { self.rim.min(self.margin(side)) } else { self.margin(side) }
    }

    /// [`Self::plate_reach`] of all four sides, as if edge bins took over every edge zone.
    pub fn reach(&self) -> [f64; 4] {
        Side::ALL.map(|s| self.plate_reach(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(w: f64, d: f64, o: PlateOptions) -> (Grid, Vec<Warning>) {
        Grid::new([w, d], &o, &MagnetSpec::default()).unwrap()
    }

    #[test]
    fn margins_and_zones_of_the_two_drawers() {
        let (g, w) = grid(250.0, 305.0, PlateOptions::default());
        assert_eq!(g.cells, [5, 7]);
        assert_eq!(g.margins, [19.5, 19.5, 5.0, 5.0]);
        assert_eq!(g.zones, [Zone::Edge, Zone::Edge, Zone::Solid, Zone::Solid]);
        // 5 + 5 mm front and back would make one 10 mm edge zone against one wall.
        assert_eq!(w, [Warning::AlignForEdgeZone { axis: 1, centred: 5.0, one_side: 10.0 }]);

        let (g, w) = grid(435.0, 330.0, PlateOptions::default());
        assert_eq!(g.cells, [10, 7]);
        assert_eq!(g.margins, [7.0, 7.0, 17.5, 17.5]);
        assert_eq!(g.zones, [Zone::Solid, Zone::Solid, Zone::Edge, Zone::Edge]);
        // 14 mm split in two is wasted; against one wall it becomes an edge zone.
        assert_eq!(w, [Warning::AlignForEdgeZone { axis: 0, centred: 7.0, one_side: 14.0 }]);

        let o = PlateOptions { align: [Align::Start, Align::Center], ..PlateOptions::default() };
        let (g, _) = grid(435.0, 330.0, o);
        assert_eq!(g.margins[..2], [0.0, 14.0]);
        assert_eq!(g.zones[1], Zone::Edge);
        assert_eq!(g.origin, [0.0, 17.5]);
    }

    #[test]
    fn plate_reach_follows_edge_mode_and_floor() {
        let (g, _) = grid(250.0, 305.0, PlateOptions::default());
        assert_eq!(g.plate_reach(Side::Left), 1.0);
        assert_eq!(g.plate_reach(Side::Front), 5.0);
        let o = PlateOptions { floor: PlateFloor::Magnets, ..PlateOptions::default() };
        assert_eq!(grid(250.0, 305.0, o).0.plate_reach(Side::Left), 19.5);
        let o = PlateOptions { edge_mode: EdgeMode::Resting, ..PlateOptions::default() };
        let (g, w) = grid(250.0, 305.0, o);
        assert_eq!(g.plate_reach(Side::Left), 19.5);
        assert_eq!(w.last(), Some(&Warning::SupportsNeeded));
    }

    #[test]
    fn without_edge_bins_every_margin_stays_solid() {
        let o = PlateOptions { edge_mode: EdgeMode::None, ..PlateOptions::default() };
        let (g, w) = grid(250.0, 305.0, o);
        assert_eq!(g.margins, [19.5, 19.5, 5.0, 5.0]);
        assert_eq!(g.zones, [Zone::Solid; 4]);
        assert_eq!(g.plate_reach(Side::Left), 19.5);
        assert!(!g.cut_back(Side::Left));
        // Nothing to gain from moving the grid against a wall, so no advice about it.
        assert!(w.is_empty());
    }

    #[test]
    fn a_surface_is_the_grid_itself() {
        let o = PlateOptions { kind: Kind::Surface, ..PlateOptions::default() };
        let (g, w) = grid(600.0, 300.0, o.clone());
        assert_eq!(g.cells, [14, 7]);
        assert_eq!(g.plate, [588.0, 294.0]);
        assert_eq!(g.margins, [0.0; 4]);
        assert_eq!(g.zones, [Zone::Solid; 4]);
        assert!(w.is_empty());
        let (g, _) = grid(600.0, 300.0, PlateOptions { cells: Some([10, 6]), ..o });
        assert_eq!(g.plate, [420.0, 252.0], "asked for fewer cells, the plate shrinks with them");
    }

    #[test]
    fn too_small_and_too_many() {
        assert!(Grid::new([40.0, 100.0], &PlateOptions::default(), &MagnetSpec::default()).is_err());
        let o = PlateOptions { cells: Some([6, 7]), ..PlateOptions::default() };
        assert!(Grid::new([250.0, 305.0], &o, &MagnetSpec::default()).is_err());
    }
}
