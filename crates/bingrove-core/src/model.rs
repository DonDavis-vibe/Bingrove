//! The data a drawer is described with. Everything here is plain serde data, shared by the
//! geometry, the command line and the plugin's tools and panel.

use serde::{Deserialize, Serialize};

use crate::spec::{BIN_SIZE, BP_H, CLEARANCE, FLOOR_Z, GRID, TAPE_ROLL_GAP, TAPE_W, bin_outer};

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Special holders that replace a plain bin on its cells.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HolderSpec {
    Tape(TapeSpec),
}

impl HolderSpec {
    pub fn cells(&self) -> [u32; 2] {
        match self {
            HolderSpec::Tape(t) => t.cells(),
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

/// How bins reaching into a wide margin are carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeMode {
    /// The plate ends just past the grid; the bin's overhang goes down to the drawer floor
    /// (prints without supports, more usable height).
    #[default]
    ToFloor,
    /// The plate keeps its solid margin; the overhang rests on it (needs supports when printing).
    Resting,
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

/// How a drawer's plate is laid out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlateOptions {
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
