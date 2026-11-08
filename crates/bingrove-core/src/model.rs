//! The data a drawer is described with. Everything here is plain serde data, shared by the
//! geometry, the command line and the plugin's tools and panel.

use serde::{Deserialize, Serialize};

use crate::spec::{BIN_SIZE, BP_H, CLEARANCE, FLOOR_Z, GRID, PEG_BASE, PEG_H, TAPE_ROLL_GAP, TAPE_W, UNIT, bin_outer};

/// Which magnet holes a bin gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagnetMode {
    #[default]
    None,
    /// Only the four outermost holes.
    Corners,
    /// Four holes under every cell.
    All,
}

/// Where divider walls go in a bin that reaches into the margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DividerMode {
    /// `dividers` compartments of equal width across the whole inside.
    #[default]
    Equal,
    /// One wall on every grid line inside the bin and one where the margin part begins.
    Grid,
}

/// One bin: footprint in cells, height in units, compartments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BinSpec {
    pub cells: [u32; 2],
    pub units: u32,
    /// Compartments along x and y (1 = no divider).
    pub dividers: [u32; 2],
    pub divider_mode: DividerMode,
    /// Stacking lip on top.
    pub lip: bool,
    pub magnets: MagnetMode,
    /// Label shelf along a wall (`label_side`, the back by default).
    pub label_tab: bool,
    pub label_side: Side,
    /// Finger scoop in percent of the possible radius (0 = none), along the walls in
    /// `scoop_sides` (the front when that is empty).
    pub scoop: u8,
    pub scoop_sides: Vec<Side>,
    /// The front wall only half as high, to reach in from the front.
    pub open_front: bool,
    /// On a wall: a storage bin with a low front and sides that slope up towards the wall, to
    /// see into and grab from (`open_front` does not apply then).
    pub hopper: bool,
    /// Round cut-outs in the top of the divider walls, to get a finger under small things.
    pub finger_cutouts: bool,
    /// A slanted window in the front wall.
    pub window: Option<Window>,
    /// Pockets for particular things, cut into a solid insert that fills the bin to the
    /// deepest pocket plus a floor.
    pub cutouts: Vec<Cutout>,
    /// Engraved on the label shelf (or the front wall without one).
    pub label_text: Option<String>,
}

/// A push-on lid for a bin: a plate with the stacking lip on top, so another bin stands on it,
/// and a plug underneath that sits in the bin's lip, with small bumps that click under it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LidSpec {
    pub cells: [u32; 2],
    /// The plate's thickness.
    pub thickness: f64,
    pub clicks: bool,
}

impl Default for LidSpec {
    fn default() -> Self {
        LidSpec { cells: [1, 1], thickness: 2.0, clicks: true }
    }
}

impl LidSpec {
    /// Outer size.
    pub fn size(&self) -> [f64; 3] {
        let [w, d] = self.cells.map(bin_outer);
        [w, d, self.thickness + crate::spec::LIP_H + crate::spec::LID_PLUG]
    }
}

/// A window cut into the front wall: `width` at the bottom, the sides slanting outwards by
/// `angle` degrees from the vertical, the bottom at about a third of the wall's height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Window {
    pub width: f64,
    pub angle: f64,
}

impl Default for Window {
    fn default() -> Self {
        Window { width: 20.0, angle: 20.0 }
    }
}

impl BinSpec {
    /// The walls that get a scoop: the listed ones, or the front.
    pub fn scoops(&self) -> Vec<Side> {
        if self.scoop == 0 {
            return Vec::new();
        }
        if self.scoop_sides.is_empty() { vec![Side::Front] } else { self.scoop_sides.clone() }
    }

    /// Whether anything tells the bin's front from its back (such a bin is never turned).
    pub fn has_a_front(&self) -> bool {
        self.label_tab
            || self.scoop > 0
            || self.open_front
            || self.hopper
            || self.window.is_some()
            || self.label_text.is_some()
    }
}

impl Default for BinSpec {
    fn default() -> Self {
        BinSpec {
            cells: [1, 1],
            units: 6,
            dividers: [1, 1],
            divider_mode: DividerMode::Equal,
            lip: true,
            magnets: MagnetMode::None,
            label_tab: false,
            label_side: Side::Back,
            scoop: 0,
            scoop_sides: vec![],
            open_front: false,
            hopper: false,
            finger_cutouts: false,
            window: None,
            cutouts: vec![],
            label_text: None,
        }
    }
}

/// A flat holder for tape rolls lying on ring posts that reach into the cardboard core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TapeSpec {
    /// Outer diameter of the largest roll.
    pub od: f64,
    /// Stacks along x and y.
    pub stacks: [u32; 2],
    /// Rolls on top of each other.
    pub height: u32,
    /// Inside diameter of the core.
    pub core: f64,
}

impl Default for TapeSpec {
    fn default() -> Self {
        TapeSpec { od: 103.0, stacks: [1, 1], height: 2, core: crate::spec::TAPE_CORE }
    }
}

impl TapeSpec {
    /// The fewest cells along x and y that hold the stacks side by side.
    pub fn cells(&self) -> [u32; 2] {
        self.stacks.map(|k| ((k as f64 * (self.od + TAPE_ROLL_GAP) + GRID - BIN_SIZE) / GRID).ceil() as u32)
    }

    /// Outer size of the holder.
    pub fn outer(&self) -> [f64; 2] {
        self.cells().map(bin_outer)
    }

    /// Largest roll that still fits, given the cells the holder ended up with.
    pub fn max_roll(&self) -> f64 {
        let [w, d] = self.outer();
        (w / self.stacks[0] as f64).min(d / self.stacks[1] as f64) - TAPE_ROLL_GAP
    }

    /// Inner height a drawer needs for the full stack.
    pub fn drawer_height(&self) -> f64 {
        FLOOR_Z + self.height as f64 * TAPE_W + 2.0
    }
}

/// Round things that stand in holes: batteries, pens, markers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "item", rename_all = "snake_case")]
pub enum RoundItem {
    Aa,
    Aaa,
    C,
    D,
    Cell18650,
    Cell21700,
    /// Anything else: diameter and height in mm.
    Custom {
        d: f64,
        h: f64,
    },
}

impl RoundItem {
    /// Diameter and height.
    pub fn size(&self) -> (f64, f64) {
        match self {
            RoundItem::Aa => (14.5, 50.5),
            RoundItem::Aaa => (10.5, 44.5),
            RoundItem::C => (26.2, 50.0),
            RoundItem::D => (34.2, 61.5),
            RoundItem::Cell18650 => (18.6, 65.0),
            RoundItem::Cell21700 => (21.2, 70.0),
            RoundItem::Custom { d, h } => (*d, *h),
        }
    }

    /// For file names.
    pub fn slug(&self) -> String {
        match self {
            RoundItem::Aa => "aa".into(),
            RoundItem::Aaa => "aaa".into(),
            RoundItem::C => "c".into(),
            RoundItem::D => "d".into(),
            RoundItem::Cell18650 => "18650".into(),
            RoundItem::Cell21700 => "21700".into(),
            RoundItem::Custom { d, .. } => format!("d{}", crate::names::mm(*d)),
        }
    }
}

/// The outline of a hole: `Round` for cylinders, `Hex` across flats (bits, hex shanks),
/// `Square` with rounded corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoleShape {
    #[default]
    Round,
    Hex,
    Square,
}

/// Standing things in a grid of holes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HolesSpec {
    pub item: RoundItem,
    pub shape: HoleShape,
    pub cells: [u32; 2],
    /// Height in units; from the item when absent (about half of it stays visible).
    pub units: Option<u32>,
    /// Air around each item.
    pub clearance: f64,
    /// Hole depth; about 55 % of the item's height when absent.
    pub depth: Option<f64>,
    /// Centre distance; item plus clearance plus wall when absent.
    pub pitch: Option<f64>,
    /// Holes across and down; as many as fit when absent.
    pub count: Option<[u32; 2]>,
    /// 45° chamfer at the mouth of each hole, mm.
    pub chamfer: f64,
    /// Engraved on the front wall.
    pub label: Option<String>,
}

impl Default for HolesSpec {
    fn default() -> Self {
        HolesSpec {
            item: RoundItem::Aa,
            shape: HoleShape::Round,
            cells: [1, 1],
            units: None,
            clearance: 0.6,
            depth: None,
            pitch: None,
            count: None,
            chamfer: 0.8,
            label: None,
        }
    }
}

impl HolesSpec {
    /// How deep the holes go.
    pub fn depth(&self) -> f64 {
        self.depth.unwrap_or(self.item.size().1 * 0.55)
    }
}

/// One hole of a row: its own diameter and the size engraved in front of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RowHole {
    pub d: f64,
    /// Hole depth; from the diameter when absent.
    pub depth: Option<f64>,
    pub label: String,
}

impl Default for RowHole {
    fn default() -> Self {
        RowHole { d: 10.0, depth: None, label: String::new() }
    }
}

impl RowHole {
    pub fn depth(&self) -> f64 {
        self.depth.unwrap_or((self.d * 0.7).clamp(8.0, 25.0))
    }
}

/// A socket's drive: the square the socket sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drive {
    Quarter,
    ThreeEighths,
    Half,
}

impl Drive {
    /// The drive square, mm.
    pub fn square(self) -> f64 {
        match self {
            Drive::Quarter => 6.35,
            Drive::ThreeEighths => 9.525,
            Drive::Half => 12.7,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Drive::Quarter => "1/4\"",
            Drive::ThreeEighths => "3/8\"",
            Drive::Half => "1/2\"",
        }
    }

    /// For file names.
    pub fn slug(self) -> &'static str {
        match self {
            Drive::Quarter => "14",
            Drive::ThreeEighths => "38",
            Drive::Half => "12",
        }
    }

    pub const ALL: [Drive; 3] = [Drive::Quarter, Drive::ThreeEighths, Drive::Half];
}

/// Rows of holes with their own sizes, each size engraved in front of its hole: sockets,
/// screwdrivers, drills, hex keys. Rows run along x; with `stepped` every row further back
/// stands one unit higher so the labels stay readable. With `pegs` the holes become square
/// pegs of that drive instead, which sockets sit on whatever their outside looks like; the
/// hole size then only spaces them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RowSpec {
    pub rows: Vec<Vec<RowHole>>,
    pub shape: HoleShape,
    pub stepped: bool,
    pub pegs: Option<Drive>,
    /// Holes leaning back by this many degrees (0: upright), for pens, hex keys, tweezers
    /// that should show their heads to the front.
    pub tilt: f64,
    /// Air around each thing.
    pub clearance: f64,
    pub cells: [u32; 2],
    pub units: Option<u32>,
    /// Engraved on the front wall, e.g. the set's name.
    pub title: Option<String>,
}

impl Default for RowSpec {
    fn default() -> Self {
        RowSpec {
            rows: vec![vec![RowHole::default()]],
            shape: HoleShape::Round,
            stepped: false,
            pegs: None,
            tilt: 0.0,
            clearance: 1.0,
            cells: [2, 1],
            units: None,
            title: None,
        }
    }
}

impl RowSpec {
    /// Deepest hole.
    pub fn max_depth(&self) -> f64 {
        self.rows.iter().flatten().map(RowHole::depth).fold(0.0, f64::max)
    }

    pub fn hole_count(&self) -> u32 {
        self.rows.iter().map(|r| r.len() as u32).sum()
    }

    /// Whether any hole or the title carries text.
    pub fn has_labels(&self) -> bool {
        self.title.is_some() || self.rows.iter().flatten().any(|h| !h.label.trim().is_empty())
    }
}

/// One place of a cradle holder: how wide and long the thing lying in it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CradleSlot {
    pub width: f64,
    pub length: f64,
    pub label: String,
}

impl Default for CradleSlot {
    fn default() -> Self {
        CradleSlot { width: 20.0, length: 120.0, label: String::new() }
    }
}

/// Sloped beds for things lying front to back (extension bars, ratchets, files), side by side
/// along x: the bed of every place rises towards the back by `angle`, so the front end is easy
/// to grab; with `stepped` each place further right stands one unit higher.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CradleSpec {
    pub slots: Vec<CradleSlot>,
    pub stepped: bool,
    /// Slope of the beds in degrees, front low.
    pub angle: f64,
    /// Air beside each thing.
    pub clearance: f64,
    pub cells: [u32; 2],
    pub units: Option<u32>,
    pub title: Option<String>,
}

impl Default for CradleSpec {
    fn default() -> Self {
        CradleSpec {
            slots: vec![CradleSlot::default()],
            stepped: false,
            angle: 8.0,
            clearance: 1.0,
            cells: [1, 3],
            units: None,
            title: None,
        }
    }
}

impl CradleSpec {
    /// Depth of a place's bed at the front: a little over half the thing.
    pub fn bed_depth(slot: &CradleSlot) -> f64 {
        (slot.width * 0.55).max(4.0)
    }

    /// How much a bed rises over `length`: the slope, but never more than three units, so a
    /// long bar does not make a tower of its holder.
    pub fn rise_over(&self, length: f64) -> f64 {
        (length * self.angle.to_radians().tan()).min(3.0 * crate::spec::UNIT)
    }

    /// How much a bed rises from front to back.
    pub fn rise(&self, slot: &CradleSlot) -> f64 {
        self.rise_over(slot.length)
    }

    /// Height the tallest place needs above the floor.
    pub fn max_height(&self) -> f64 {
        let steps = if self.stepped { self.slots.len().saturating_sub(1) as f64 } else { 0.0 };
        self.slots.iter().map(|s| Self::bed_depth(s) + self.rise(s) + 2.0).fold(0.0, f64::max)
            + steps * crate::spec::UNIT
    }
}

/// One slot of a V-slot holder: how thick and how tall the thing standing in it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VSlot {
    pub width: f64,
    pub depth: f64,
    pub label: String,
}

impl Default for VSlot {
    fn default() -> Self {
        VSlot { width: 14.0, depth: 35.0, label: String::new() }
    }
}

/// Slots with a V-shaped bottom, side by side along x, for pliers, scissors and wrenches
/// standing on edge: the V centres things thinner than the slot. `closed_end` keeps the back
/// wall; `ramp` lets the slot bottoms rise towards the back, so wrenches sorted by size show
/// their heads in a row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VSlotSpec {
    pub slots: Vec<VSlot>,
    pub closed_end: bool,
    pub ramp: bool,
    /// Air beside each thing.
    pub clearance: f64,
    pub cells: [u32; 2],
    pub units: Option<u32>,
    pub title: Option<String>,
}

impl Default for VSlotSpec {
    fn default() -> Self {
        VSlotSpec {
            slots: vec![VSlot::default(); 3],
            closed_end: true,
            ramp: false,
            clearance: 1.0,
            cells: [2, 2],
            units: None,
            title: None,
        }
    }
}

impl VSlotSpec {
    pub fn max_depth(&self) -> f64 {
        self.slots.iter().map(|s| s.depth).fold(0.0, f64::max) + if self.ramp { RAMP_RISE } else { 0.0 }
    }
}

/// How much a ramped V-slot bottom rises from front to back.
pub const RAMP_RISE: f64 = 14.0;

/// Rings standing on the bin floor for round things to sit in: bottles, spray cans, cable
/// spools, glue sticks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RingSpec {
    /// Diameter of the thing.
    pub d: f64,
    /// Height of the rings.
    pub height: f64,
    /// Rings across and down; as many as fit when absent.
    pub count: Option<[u32; 2]>,
    pub clearance: f64,
    pub cells: [u32; 2],
    pub units: Option<u32>,
    pub label: Option<String>,
}

impl Default for RingSpec {
    fn default() -> Self {
        RingSpec { d: 66.0, height: 25.0, count: None, clearance: 1.0, cells: [2, 2], units: None, label: None }
    }
}

/// Parallel slots across the bin for flat things standing on edge: cards, swatches, blades,
/// coin cells. Slots run along y and are spaced along x.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SlotSpec {
    /// Slot width (the thing's thickness plus air).
    pub width: f64,
    pub depth: f64,
    /// Centre distance.
    pub pitch: f64,
    /// Slot length along y; the bin's inside when absent.
    pub length: Option<f64>,
    /// Slots across; as many as fit when absent.
    pub count: Option<u32>,
    pub cells: [u32; 2],
    pub units: Option<u32>,
    pub label: Option<String>,
}

impl Default for SlotSpec {
    fn default() -> Self {
        SlotSpec {
            width: 3.0,
            depth: 20.0,
            pitch: 6.0,
            length: None,
            count: None,
            cells: [2, 1],
            units: None,
            label: None,
        }
    }
}

/// 1/4" hex bits standing in hex holes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BitsSpec {
    pub cells: [u32; 2],
    pub units: Option<u32>,
}

impl Default for BitsSpec {
    fn default() -> Self {
        BitsSpec { cells: [1, 1], units: None }
    }
}

/// Lying things in troughs across the bin: pens, screwdrivers (shaft and a wider handle).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TroughSpec {
    /// Diameter of the thing (the shaft, for screwdrivers).
    pub diameter: f64,
    /// Length of the thing; it may stick out over the bin.
    pub length: f64,
    /// Diameter of a handle at the back, for screwdrivers.
    pub handle: Option<f64>,
    /// Troughs across the bin; as many as fit when absent.
    pub count: Option<u32>,
    pub cells: [u32; 2],
    pub units: Option<u32>,
}

impl Default for TroughSpec {
    fn default() -> Self {
        TroughSpec { diameter: 10.0, length: 150.0, handle: None, count: None, cells: [3, 1], units: None }
    }
}

/// Cells across a wall part needs for `width` mm.
fn wall_cells_x(width: f64) -> u32 {
    ((width + GRID - BIN_SIZE) / GRID - 1e-9).ceil().max(1.0) as u32
}

/// Cells up the wall a wall part needs for something `height` mm tall from its bottom.
fn wall_cells_y(height: f64) -> u32 {
    ((height + crate::spec::HOOK_LIFT + GRID - BIN_SIZE) / GRID - 1e-9).ceil().max(1.0) as u32
}

/// Units a wall part needs to reach `depth` mm out of the wall in front of its feet.
fn wall_units(depth: f64) -> u32 {
    ((depth + crate::spec::FOOT_H + crate::spec::FLOOR) / UNIT - 1e-9).ceil().max(2.0) as u32
}

/// Standing things in holes on a wall: a block at the bottom of the part with a grid of holes
/// from above, `count` across and out of the wall, leaning `tilt` degrees away from it at the
/// top. Screwdrivers, pens, files, extension bars of one size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WallHolesSpec {
    /// Diameter of the thing (across flats for hex).
    pub d: f64,
    /// How deep it stands in.
    pub depth: f64,
    pub shape: HoleShape,
    /// Holes across and rows out of the wall.
    pub count: [u32; 2],
    pub tilt: f64,
    /// Holes open at the bottom: the thing hangs through by a wider top (screwdrivers).
    pub through: bool,
    pub clearance: f64,
    pub cells: [u32; 2],
    pub units: Option<u32>,
    pub title: Option<String>,
}

impl Default for WallHolesSpec {
    fn default() -> Self {
        WallHolesSpec {
            d: 10.0,
            depth: 25.0,
            shape: HoleShape::Round,
            count: [4, 1],
            tilt: 0.0,
            through: false,
            clearance: 1.0,
            cells: [1, 1],
            units: None,
            title: None,
        }
    }
}

impl WallHolesSpec {
    /// The widest extent of one hole.
    pub fn hole(&self) -> f64 {
        crate::catalog::across(self.shape, self.d + self.clearance)
    }

    pub fn pitch(&self) -> f64 {
        self.hole() + crate::catalog::HOLE_WALL
    }

    fn tilt(&self) -> f64 {
        self.tilt.clamp(0.0, 30.0).to_radians()
    }

    /// Width, depth out of the wall and height of the block with the holes.
    pub fn block(&self) -> [f64; 3] {
        let wall = crate::catalog::HOLE_WALL;
        let n = self.count.map(|c| c.max(1) as f64);
        [
            n[0] * self.pitch() + wall,
            (n[1] - 1.0) * self.pitch() + self.hole() + self.depth * self.tilt().sin() + 2.0 * wall,
            if self.through { self.depth.max(6.0) } else { crate::spec::FLOOR + self.depth * self.tilt().cos() + 1.0 },
        ]
    }

    /// The smallest part for the holes.
    pub fn fit(mut self) -> Self {
        let [w, _, h] = self.block();
        self.cells = [wall_cells_x(w), wall_cells_y(h)];
        self
    }
}

/// Bars standing in short sockets on a wall, each its own size, leaning `angle` degrees towards
/// the wall at the top so they rest against it: extension bars, punches, chisels. With
/// `stepped` every place further right stands a unit higher.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WallCradleSpec {
    /// Per place: the bar's diameter, how deep it stands in, the text in front.
    pub slots: Vec<RowHole>,
    pub angle: f64,
    pub stepped: bool,
    pub clearance: f64,
    pub cells: [u32; 2],
    pub units: Option<u32>,
    pub title: Option<String>,
}

impl Default for WallCradleSpec {
    fn default() -> Self {
        WallCradleSpec {
            slots: vec![RowHole { d: 12.0, depth: Some(20.0), label: String::new() }; 3],
            angle: 12.0,
            stepped: true,
            clearance: 1.0,
            cells: [1, 1],
            units: None,
            title: None,
        }
    }
}

impl WallCradleSpec {
    fn angle(&self) -> f64 {
        self.angle.clamp(0.0, 30.0).to_radians()
    }

    /// Width of a place: the socket and a wall.
    pub fn place(&self, slot: &RowHole) -> f64 {
        slot.d + self.clearance + crate::catalog::HOLE_WALL
    }

    /// Top of place `k` above the part's bottom.
    pub fn top(&self, k: usize) -> f64 {
        let base = self
            .slots
            .iter()
            .map(|s| crate::spec::FLOOR + s.depth() * self.angle().cos() + 1.0)
            .fold(crate::spec::FLOOR + 6.0, f64::max);
        base + if self.stepped { k as f64 * UNIT } else { 0.0 }
    }

    /// Width, depth out of the wall and height of the places together.
    pub fn block(&self) -> [f64; 3] {
        let wall = crate::catalog::HOLE_WALL;
        let width = self.slots.iter().map(|s| self.place(s)).sum::<f64>() + wall;
        let depth = self
            .slots
            .iter()
            .map(|s| s.d + self.clearance + s.depth() * self.angle().sin() + 2.0 * wall + 0.5)
            .fold(0.0, f64::max);
        [width, depth, self.top(self.slots.len().saturating_sub(1))]
    }

    pub fn fit(mut self) -> Self {
        let [w, _, h] = self.block();
        self.cells = [wall_cells_x(w), wall_cells_y(h + 3.0)];
        self
    }
}

/// Things standing jaws down in V-bottomed slots on a shelf out of the wall, handles up:
/// pliers, scissors, wrenches. The slots run out from the wall, open at the front unless
/// `closed_front`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WallVSlotSpec {
    pub slots: Vec<VSlot>,
    pub closed_front: bool,
    /// How far the slots reach out of the wall.
    pub length: f64,
    pub clearance: f64,
    pub cells: [u32; 2],
    pub units: Option<u32>,
    pub title: Option<String>,
}

impl Default for WallVSlotSpec {
    fn default() -> Self {
        WallVSlotSpec {
            slots: vec![VSlot { width: 14.0, depth: 25.0, label: String::new() }; 3],
            closed_front: false,
            length: 30.0,
            clearance: 1.0,
            cells: [1, 1],
            units: None,
            title: None,
        }
    }
}

impl WallVSlotSpec {
    /// Width, depth out of the wall and height of the shelf with the slots.
    pub fn block(&self) -> [f64; 3] {
        let wall = crate::catalog::HOLE_WALL;
        let width = self.slots.iter().map(|s| s.width + self.clearance + wall).sum::<f64>() + wall;
        let deepest = self.slots.iter().map(|s| s.depth).fold(0.0, f64::max);
        let front = if self.closed_front { crate::spec::WALL } else { 0.0 };
        [width, self.length + front, crate::spec::FLOOR + deepest + 1.0]
    }

    pub fn fit(mut self) -> Self {
        let [w, _, h] = self.block();
        self.cells = [wall_cells_x(w), wall_cells_y(h + 3.0)];
        self
    }
}

/// Special holders that replace a plain bin on its cells.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HolderSpec {
    Tape(TapeSpec),
    Holes(HolesSpec),
    Bits(BitsSpec),
    Troughs(TroughSpec),
    Row(RowSpec),
    Slots(SlotSpec),
    Cradles(CradleSpec),
    // "vslots" is what the tools tell the AI; "v_slots" is how 0.3 wrote it.
    #[serde(rename = "vslots", alias = "v_slots")]
    VSlots(VSlotSpec),
    Rings(RingSpec),
    WallHoles(WallHolesSpec),
    WallCradles(WallCradleSpec),
    #[serde(rename = "wall_vslots", alias = "wall_v_slots")]
    WallVSlots(WallVSlotSpec),
}

/// Air around hex bits (across flats) and a 1/4" bit's size.
pub const BIT_ACROSS_FLATS: f64 = 6.35;
pub const BIT_CLEARANCE: f64 = 0.3;
pub const BIT_DEPTH: f64 = 9.0;

impl HolderSpec {
    pub fn cells(&self) -> [u32; 2] {
        match self {
            HolderSpec::Tape(t) => t.cells(),
            HolderSpec::Holes(h) => h.cells,
            HolderSpec::Bits(b) => b.cells,
            HolderSpec::Troughs(t) => t.cells,
            HolderSpec::Row(r) => r.cells,
            HolderSpec::Slots(s) => s.cells,
            HolderSpec::Cradles(c) => c.cells,
            HolderSpec::VSlots(v) => v.cells,
            HolderSpec::Rings(r) => r.cells,
            HolderSpec::WallHoles(w) => w.cells,
            HolderSpec::WallCradles(w) => w.cells,
            HolderSpec::WallVSlots(w) => w.cells,
        }
    }

    /// Whether the holder hangs on a wall (made lying on its bottom face, hooks in its feet).
    pub fn is_wall(&self) -> bool {
        matches!(self, HolderSpec::WallHoles(_) | HolderSpec::WallCradles(_) | HolderSpec::WallVSlots(_))
    }

    /// The holder with another footprint (tape decides its own).
    pub fn with_cells(mut self, cells: [u32; 2]) -> Self {
        match &mut self {
            HolderSpec::Tape(_) => {}
            HolderSpec::Holes(h) => h.cells = cells,
            HolderSpec::Bits(b) => b.cells = cells,
            HolderSpec::Troughs(t) => t.cells = cells,
            HolderSpec::Row(r) => r.cells = cells,
            HolderSpec::Slots(s) => s.cells = cells,
            HolderSpec::Cradles(c) => c.cells = cells,
            HolderSpec::VSlots(v) => v.cells = cells,
            HolderSpec::Rings(r) => r.cells = cells,
            HolderSpec::WallHoles(w) => w.cells = cells,
            HolderSpec::WallCradles(w) => w.cells = cells,
            HolderSpec::WallVSlots(w) => w.cells = cells,
        }
        self
    }

    /// Asks for a height in units (`None`: from the pattern again); tape keeps its own height.
    pub fn set_units(&mut self, units: Option<u32>) {
        match self {
            HolderSpec::Tape(_) => {}
            HolderSpec::Holes(h) => h.units = units,
            HolderSpec::Bits(b) => b.units = units,
            HolderSpec::Troughs(t) => t.units = units,
            HolderSpec::Row(r) => r.units = units,
            HolderSpec::Slots(s) => s.units = units,
            HolderSpec::Cradles(c) => c.units = units,
            HolderSpec::VSlots(v) => v.units = units,
            HolderSpec::Rings(r) => r.units = units,
            HolderSpec::WallHoles(w) => w.units = units,
            HolderSpec::WallCradles(w) => w.units = units,
            HolderSpec::WallVSlots(w) => w.units = units,
        }
    }

    /// Height of the holder in units; tape holders have their own height (0).
    pub fn units(&self) -> u32 {
        let from_depth = |depth: f64| ((FLOOR_Z + depth + 2.0) / UNIT).ceil().max(2.0) as u32;
        match self {
            HolderSpec::Tape(_) => 0,
            HolderSpec::Holes(h) => h.units.unwrap_or_else(|| from_depth(h.depth())),
            HolderSpec::Bits(b) => b.units.unwrap_or_else(|| from_depth(BIT_DEPTH)),
            HolderSpec::Troughs(t) => t.units.unwrap_or_else(|| from_depth(t.handle.unwrap_or(t.diameter) + 1.0)),
            HolderSpec::Row(r) => r.units.unwrap_or_else(|| {
                let steps = if r.stepped { r.rows.len().saturating_sub(1) as f64 } else { 0.0 };
                // Pegs stand on a thin base and reach PEG_H above it; holes go down instead.
                let reach = if r.pegs.is_some() { PEG_BASE + PEG_H } else { r.max_depth() };
                from_depth(reach + steps * UNIT)
            }),
            HolderSpec::Slots(s) => s.units.unwrap_or_else(|| from_depth(s.depth)),
            HolderSpec::Cradles(c) => c.units.unwrap_or_else(|| from_depth(c.max_height())),
            HolderSpec::VSlots(v) => v.units.unwrap_or_else(|| from_depth(v.max_depth())),
            // Rings stand on the floor; the shell is as tall as they are.
            HolderSpec::Rings(r) => r.units.unwrap_or_else(|| from_depth(r.height - 2.0)),
            // Wall holders reach out of the wall as far as their block.
            HolderSpec::WallHoles(w) => w.units.unwrap_or_else(|| wall_units(w.block()[1])),
            HolderSpec::WallCradles(w) => w.units.unwrap_or_else(|| wall_units(w.block()[1])),
            HolderSpec::WallVSlots(w) => w.units.unwrap_or_else(|| wall_units(w.block()[1])),
        }
    }

    /// Outer size of the holder.
    pub fn size(&self) -> [f64; 3] {
        let [w, d] = self.cells().map(bin_outer);
        match self {
            HolderSpec::Tape(t) => [w, d, FLOOR_Z + crate::spec::TAPE_POST_H + (t.height as f64 - 1.0) * TAPE_W],
            h if h.is_wall() => crate::spec::wall_size(self.cells(), self.units()),
            _ => [w, d, self.units() as f64 * UNIT + crate::spec::LIP_H],
        }
    }

    /// For file names.
    pub fn slug(&self) -> String {
        match self {
            HolderSpec::Tape(_) => "tape".into(),
            HolderSpec::Holes(h) => {
                let shape = match h.shape {
                    HoleShape::Round => "",
                    HoleShape::Hex => "_hex",
                    HoleShape::Square => "_eckig",
                };
                match h.item {
                    RoundItem::Custom { .. } => format!("loch_{}{shape}", h.item.slug()),
                    _ => format!("akku_{}", h.item.slug()),
                }
            }
            HolderSpec::Bits(_) => "bits".into(),
            HolderSpec::Troughs(t) => match t.handle {
                Some(_) => format!("schraubendreher_d{}", crate::names::mm(t.diameter)),
                None => format!("liegend_d{}", crate::names::mm(t.diameter)),
            },
            HolderSpec::Row(r) => {
                let title = r.title.as_deref().map(crate::names::slug).filter(|s| !s.is_empty());
                let what = title.unwrap_or_else(|| format!("{}loch", r.hole_count()));
                let pegs = r.pegs.map(|d| format!("_stecker{}", d.slug())).unwrap_or_default();
                let tilt = if r.tilt > 0.0 { "_schraeg" } else { "" };
                format!("reihe_{what}{pegs}{tilt}{}", if r.stepped { "_gestuft" } else { "" })
            }
            HolderSpec::Slots(s) => format!("schlitze_{}", crate::names::mm(s.width)),
            HolderSpec::Cradles(c) => {
                let title = c.title.as_deref().map(crate::names::slug).filter(|s| !s.is_empty());
                let what = title.unwrap_or_else(|| format!("{}", c.slots.len()));
                format!("wiegen_{what}{}", if c.stepped { "_gestuft" } else { "" })
            }
            HolderSpec::VSlots(v) => {
                let title = v.title.as_deref().map(crate::names::slug).filter(|s| !s.is_empty());
                let what = title.unwrap_or_else(|| format!("{}", v.slots.len()));
                format!("vschlitze_{what}{}", if v.ramp { "_rampe" } else { "" })
            }
            HolderSpec::Rings(r) => format!("ringe_d{}", crate::names::mm(r.d)),
            HolderSpec::WallHoles(w) => {
                let title = w.title.as_deref().map(crate::names::slug).filter(|s| !s.is_empty());
                let what = title.unwrap_or_else(|| format!("d{}", crate::names::mm(w.d)));
                let shape = match w.shape {
                    HoleShape::Round => "",
                    HoleShape::Hex => "_hex",
                    HoleShape::Square => "_eckig",
                };
                let tilt = if w.tilt > 0.0 { "_schraeg" } else { "" };
                let through = if w.through { "_durch" } else { "" };
                format!("loecher_{what}{shape}_{}x{}{tilt}{through}", w.count[0], w.count[1])
            }
            HolderSpec::WallCradles(w) => {
                let title = w.title.as_deref().map(crate::names::slug).filter(|s| !s.is_empty());
                let what = title.unwrap_or_else(|| format!("{}", w.slots.len()));
                format!("wiegen_{what}{}", if w.stepped { "_gestuft" } else { "" })
            }
            HolderSpec::WallVSlots(w) => {
                let title = w.title.as_deref().map(crate::names::slug).filter(|s| !s.is_empty());
                let what = title.unwrap_or_else(|| format!("{}", w.slots.len()));
                format!("vschlitze_{what}{}", if w.closed_front { "_zu" } else { "" })
            }
        }
    }
}

/// A pocket cut into a bin's solid insert, for one particular thing. Positions are mm from the
/// bin's outer front-left corner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum Cutout {
    /// A rounded box, `at` its centre.
    Box {
        at: [f64; 2],
        size: [f64; 2],
        depth: f64,
    },
    Cylinder {
        at: [f64; 2],
        d: f64,
        depth: f64,
    },
    /// An outline (counter-clockwise), grown by `clearance`.
    Polygon {
        points: Vec<[f64; 2]>,
        clearance: f64,
        depth: f64,
    },
}

impl Cutout {
    pub fn depth(&self) -> f64 {
        match self {
            Cutout::Box { depth, .. } | Cutout::Cylinder { depth, .. } | Cutout::Polygon { depth, .. } => *depth,
        }
    }
}

/// The baseplate of a drawer in the original script's terms: centred grid, solid margins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlateSpec {
    /// Inner size of the drawer (x = width, y = depth from front to back).
    pub drawer: [f64; 2],
    /// Cells per piece along x and along y (front to back); their sums are the grid size.
    pub split: [Vec<u32>; 2],
    /// Floor with a magnet under each cell corner.
    pub magnets: bool,
}

impl Default for PlateSpec {
    fn default() -> Self {
        PlateSpec { drawer: [0.0, 0.0], split: [vec![], vec![]], magnets: false }
    }
}

impl PlateSpec {
    pub fn cells(&self) -> [u32; 2] {
        [self.split[0].iter().sum(), self.split[1].iter().sum()]
    }

    /// Outline of the plate: the drawer minus the clearance.
    pub fn size(&self) -> [f64; 2] {
        self.drawer.map(|d| d - CLEARANCE)
    }

    /// Solid margin around the centred grid, per side, along x and y.
    pub fn margins(&self) -> [f64; 2] {
        let (size, cells) = (self.size(), self.cells());
        [0, 1].map(|k| (size[k] - cells[k] as f64 * GRID) / 2.0)
    }

    /// Plate height; a magnet floor lifts the grid.
    pub fn height(&self, magnet: &crate::profile::MagnetSpec) -> f64 {
        BP_H + if self.magnets { magnet.h + magnet.floor } else { 0.0 }
    }

    /// The same plate as drawer options: centred, every margin solid, the given split.
    pub fn options(&self) -> PlateOptions {
        PlateOptions {
            clearance: CLEARANCE,
            align: [Align::Center; 2],
            edge_threshold: f64::INFINITY,
            floor: if self.magnets { PlateFloor::Magnets } else { PlateFloor::Open },
            split: Some(self.split.clone()),
            cells: Some(self.cells()),
            ..PlateOptions::default()
        }
    }
}

/// Sides of the drawer as seen from above with the front at the bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left = 0,
    Right = 1,
    Front = 2,
    Back = 3,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Left, Side::Right, Side::Front, Side::Back];

    /// German initial, used in file names (links, rechts, vorne, hinten).
    pub fn letter(self) -> char {
        ['L', 'R', 'V', 'H'][self as usize]
    }

    pub fn german(self) -> &'static str {
        ["links", "rechts", "vorne", "hinten"][self as usize]
    }
}

/// Where the leftover space of an axis goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    /// Grid against the left/front wall, all leftover on the right/back.
    Start,
    #[default]
    Center,
    End,
}

/// Whether bins reach into a wide margin, and how the overhang is carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeMode {
    /// The plate ends just past the grid; the bin's overhang goes down to the drawer floor
    /// (prints without supports, more usable height).
    #[default]
    ToFloor,
    /// The plate keeps its solid margin; the overhang rests on it (needs supports when printing).
    Resting,
    /// No edge bins: every margin stays solid plate (hollowed from below when wide enough) and
    /// every bin is a plain 42 n one, whatever the threshold says.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlateFloor {
    /// Open frame, bins stand on the drawer floor.
    #[default]
    Open,
    /// A solid floor without holes; what screws and connectors need.
    Solid,
    /// Floor with a magnet under each cell corner.
    Magnets,
}

/// What the plate is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// An insert for a drawer: the plate fills the inner size, bins cover the grid.
    #[default]
    Drawer,
    /// A surface such as a workbench top: the plate is the grid itself, no walls around it,
    /// no margins, and bins stand only where something is put.
    Surface,
    /// A plate on a wall: the grid itself like a surface, y running up the wall. Parts hang on a
    /// hook in every cell and are built to be printed lying on their bottom face.
    Wall,
}

/// Whether bins have to cover every cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cover {
    /// Every cell exactly once; gaps are filled with bins.
    #[default]
    Exact,
    /// Cells may stay free (the plate shows); only overlaps and bins outside the grid are wrong.
    Partial,
}

/// How the plate holds to a surface and to its neighbouring pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Mount {
    /// Countersunk screw holes in the cell corners.
    pub screws: bool,
    /// Dovetail connectors along the cut edges between pieces.
    pub connectors: bool,
    /// Pockets in the top edge of a wall plate for hooks that hang it on an IKEA Skadis board.
    pub skadis: bool,
}

/// What the floor of a plate looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlateStyle {
    /// A cross cut out of every cell's floor, leaving the four corner pads: light and quick.
    #[default]
    Skeleton,
    /// The whole floor: stiffer, screws anywhere, magnets below everywhere.
    Full,
}

/// Everything the plate's geometry depends on besides the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlateBuild {
    pub floor: PlateFloor,
    pub style: PlateStyle,
    pub mount: Mount,
    /// Magnet pockets in the underside of the pads, for steel surfaces and tool carts.
    pub magnets_below: bool,
    /// A wall plate (always with a full floor).
    pub wall: bool,
    /// A hook on every cell's floor that wall parts hang on.
    pub hooks: bool,
}

/// How a drawer's plate is laid out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlateOptions {
    pub kind: Kind,
    pub cover: Cover,
    pub mount: Mount,
    pub style: PlateStyle,
    pub magnets_below: bool,
    /// Plate = drawer minus this, per axis.
    pub clearance: f64,
    pub align: [Align; 2],
    /// Margins at least this wide become edge zones that bins reach into.
    pub edge_threshold: f64,
    pub edge_mode: EdgeMode,
    pub floor: PlateFloor,
    /// Cells per plate piece along x and y; computed from the printer bed when absent.
    pub split: Option<[Vec<u32>; 2]>,
    /// Fewer cells than fit, e.g. to leave room for something else; as many as fit when absent.
    pub cells: Option<[u32; 2]>,
    /// Width of the flat top the plate keeps past the outer grid line in a ToFloor edge zone.
    pub rim: f64,
    /// A wall plate's hooks (wall parts get the keyholes to hang on them); without them parts
    /// hold by magnets alone and any bin fits.
    pub hooks: bool,
}

impl Default for PlateOptions {
    fn default() -> Self {
        PlateOptions {
            kind: Kind::Drawer,
            cover: Cover::Exact,
            mount: Mount::default(),
            style: PlateStyle::Skeleton,
            magnets_below: false,
            clearance: CLEARANCE,
            align: [Align::Center; 2],
            edge_threshold: 10.0,
            edge_mode: EdgeMode::ToFloor,
            floor: PlateFloor::Open,
            split: None,
            cells: None,
            rim: 1.0,
            hooks: true,
        }
    }
}

impl PlateOptions {
    /// What the plate is built with: an open frame holds neither screws, connectors nor magnets
    /// below.
    pub fn build(&self) -> PlateBuild {
        if self.kind == Kind::Wall {
            // A wall plate carries the parts on its floor: always floored and full, never with
            // magnets below (the wall is behind it).
            let floor = if self.floor == PlateFloor::Open { PlateFloor::Solid } else { self.floor };
            return PlateBuild {
                floor,
                style: PlateStyle::Full,
                mount: self.mount,
                magnets_below: false,
                wall: true,
                hooks: self.hooks,
            };
        }
        let floored = self.floor != PlateFloor::Open;
        PlateBuild {
            floor: self.floor,
            style: self.style,
            mount: if floored { Mount { skadis: false, ..self.mount } } else { Mount::default() },
            magnets_below: floored && self.magnets_below,
            wall: false,
            hooks: false,
        }
    }
}

/// Which margins a placement may reach into; every side it touches by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EdgeUse {
    /// Sides (left, right, front, back) where this bin leaves the margin to the plate.
    pub off: [bool; 4],
}

impl EdgeUse {
    fn is_default(&self) -> bool {
        *self == EdgeUse::default()
    }
}

/// A bin or holder placed on the grid; cell (0, 0) is front left.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub x: u32,
    pub y: u32,
    #[serde(flatten)]
    pub bin: BinSpec,
    /// Replaces the bin; `bin.cells` then has to be the holder's cells.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<HolderSpec>,
    #[serde(default, skip_serializing_if = "EdgeUse::is_default")]
    pub edges: EdgeUse,
    /// What goes in, shown in plans and lists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// A lid for this bin is printed too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lid: bool,
}

impl Placement {
    pub fn bin(x: u32, y: u32, bin: BinSpec) -> Self {
        Placement { x, y, bin, holder: None, edges: EdgeUse::default(), label: None, lid: false }
    }

    /// Footprint in cells.
    pub fn cells(&self) -> [u32; 2] {
        self.holder.as_ref().map_or(self.bin.cells, HolderSpec::cells)
    }
}

/// The drawer itself: inner size, inner height (0 = unknown).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DrawerSpec {
    pub name: String,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
}

impl Default for DrawerSpec {
    fn default() -> Self {
        DrawerSpec { name: "Schublade".into(), width: 0.0, depth: 0.0, height: 0.0 }
    }
}

/// The printer the plate is split for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BedSpec {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Free border kept on the bed.
    pub margin: f64,
}

impl Default for BedSpec {
    fn default() -> Self {
        BedSpec { name: "Anycubic Kobra S1".into(), x: 250.0, y: 250.0, z: 250.0, margin: 5.0 }
    }
}

impl BedSpec {
    /// Whether a footprint fits, turned by 90° if needed.
    pub fn fits(&self, w: f64, d: f64) -> bool {
        let (bx, by) = (self.x - 2.0 * self.margin, self.y - 2.0 * self.margin);
        (w <= bx + 1e-9 && d <= by + 1e-9) || (w <= by + 1e-9 && d <= bx + 1e-9)
    }
}

/// Something that should go into the drawer, in the user's or an AI's words turned into numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default = "one")]
    pub quantity: u32,
    #[serde(flatten)]
    pub what: ItemKind,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemKind {
    /// A bin of a given size; height from the drawer when `units` is absent.
    Bin {
        cells: [u32; 2],
        #[serde(default)]
        units: Option<u32>,
        #[serde(default = "no_dividers")]
        dividers: [u32; 2],
    },
    /// An object to hold: outer size x, y, z in mm; the bin is sized around it.
    Object {
        size: [f64; 3],
        #[serde(default = "object_clearance")]
        clearance: f64,
    },
    Tape(TapeSpec),
    /// A holder with its own pattern, or a set of sizes from the catalogue by `set`; cells from
    /// `count` when the holder's cells are left at 1x1.
    Holder {
        #[serde(default)]
        holder: Option<HolderSpec>,
        #[serde(default)]
        set: Option<String>,
        #[serde(default)]
        count: Option<u32>,
    },
}

fn no_dividers() -> [u32; 2] {
    [1, 1]
}

fn object_clearance() -> f64 {
    2.0
}

/// Everything needed to make a drawer insert; saved next to the generated files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DrawerProject {
    pub version: u32,
    pub drawer: DrawerSpec,
    pub options: PlateOptions,
    pub placements: Vec<Placement>,
    /// Wishes the solver turns into placements when `placements` is empty.
    pub items: Vec<ItemRequest>,
    pub profile: crate::profile::Profile,
    pub bed: BedSpec,
}

impl Default for DrawerProject {
    fn default() -> Self {
        DrawerProject {
            version: 1,
            drawer: DrawerSpec::default(),
            options: PlateOptions::default(),
            placements: vec![],
            items: vec![],
            profile: crate::profile::Profile::default(),
            bed: BedSpec::default(),
        }
    }
}
