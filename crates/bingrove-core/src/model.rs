//! The data a drawer is described with. Everything here is plain serde data, shared by the
//! geometry, the command line and the plugin's tools and panel.

use serde::{Deserialize, Serialize};

use crate::spec::{BIN_SIZE, BP_H, CLEARANCE, FLOOR_Z, GRID, TAPE_ROLL_GAP, TAPE_W, UNIT, bin_outer};

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
    /// Label shelf along the back wall.
    pub label_tab: bool,
    /// Finger scoop along the front wall, in percent of the possible radius (0 = none).
    pub scoop: u8,
    /// Pockets for particular things, cut into a solid insert that fills the bin to the
    /// deepest pocket plus a floor.
    pub cutouts: Vec<Cutout>,
    /// Engraved on the label shelf (or the front wall without one).
    pub label_text: Option<String>,
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
            scoop: 0,
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

/// Rows of holes with their own sizes, each size engraved in front of its hole: sockets,
/// screwdrivers, drills, hex keys. Rows run along x; with `stepped` every row further back
/// stands one unit higher so the labels stay readable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RowSpec {
    pub rows: Vec<Vec<RowHole>>,
    pub shape: HoleShape,
    pub stepped: bool,
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
        }
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
        }
        self
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
                from_depth(r.max_depth() + steps * UNIT)
            }),
            HolderSpec::Slots(s) => s.units.unwrap_or_else(|| from_depth(s.depth)),
        }
    }

    /// Outer size of the holder.
    pub fn size(&self) -> [f64; 3] {
        let [w, d] = self.cells().map(bin_outer);
        match self {
            HolderSpec::Tape(t) => [w, d, FLOOR_Z + crate::spec::TAPE_POST_H + (t.height as f64 - 1.0) * TAPE_W],
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
                format!("reihe_{what}{}", if r.stepped { "_gestuft" } else { "" })
            }
            HolderSpec::Slots(s) => format!("schlitze_{}", crate::names::mm(s.width)),
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
}

/// How a drawer's plate is laid out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlateOptions {
    pub kind: Kind,
    pub cover: Cover,
    pub mount: Mount,
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
}

impl Default for PlateOptions {
    fn default() -> Self {
        PlateOptions {
            kind: Kind::Drawer,
            cover: Cover::Exact,
            mount: Mount::default(),
            clearance: CLEARANCE,
            align: [Align::Center; 2],
            edge_threshold: 10.0,
            edge_mode: EdgeMode::ToFloor,
            floor: PlateFloor::Open,
            split: None,
            cells: None,
            rim: 1.0,
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
}

impl Placement {
    pub fn bin(x: u32, y: u32, bin: BinSpec) -> Self {
        Placement { x, y, bin, holder: None, edges: EdgeUse::default(), label: None }
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
