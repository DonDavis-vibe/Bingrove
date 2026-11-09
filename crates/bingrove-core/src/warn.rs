//! Things a plan wants the user (or the AI) to know. Codes are stable; the GUI translates them.

use serde::{Deserialize, Serialize};

use crate::model::Side;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Warning {
    /// Centred, both margins of an axis stay too narrow to use; against one wall one wide margin
    /// would become an edge zone.
    AlignForEdgeZone { axis: usize, centred: f64, one_side: f64 },
    /// Resting edge bins overhang without support underneath.
    SupportsNeeded,
    /// An open plate cut back for ToFloor edge bins touches no drawer wall on those sides;
    /// the bins hold it in place.
    PlateHeldByBins { sides: Vec<Side> },
    /// A part is larger than the printer bed.
    PartTooLarge { part: String, size: [f64; 2] },
    /// An item needs more height than the drawer has.
    ItemTooTall { item: String, units: u32, max_units: u32 },
    /// No free spot for an item.
    ItemUnplaced { item: String, cells: [u32; 2] },
    /// The drawer height is unknown, bins get a default height.
    HeightUnknown { units: u32 },
    /// A holder made for a wall stands in a drawer or on a surface.
    WallHolderOffWall { holder: String },
    /// A holder for drawers hangs on a wall: its pockets open to the front and the plate's
    /// hooks keep its feet out.
    HolderNotForWall { holder: String },
    /// A surface or a wall was given more cells than its size holds: the plate sticks out
    /// along `axis` (0 across, 1 deep or up the wall).
    GridLargerThanSpace { axis: usize, grid: f64, space: f64, wall: bool },
}

/// What a surface's or a wall's size along an axis is called.
fn extent(axis: usize, wall: bool, german: bool) -> &'static str {
    match (axis, wall, german) {
        (0, _, true) => "breit",
        (0, _, false) => "wide",
        (_, true, true) => "hoch",
        (_, true, false) => "high",
        (_, false, true) => "tief",
        (_, false, false) => "deep",
    }
}

impl Warning {
    /// The warning in German, for a German UI.
    pub fn german(&self) -> String {
        match self {
            Warning::AlignForEdgeZone { axis, centred, one_side } => format!(
                "{} Ränder von {centred:.1} mm sind zu schmal zum Nutzen; an eine Wand gerückt wird ein Rand von {one_side:.1} mm zur Randzone",
                if *axis == 0 { "Linker/rechter" } else { "Vorderer/hinterer" }
            ),
            Warning::SupportsNeeded => "Aufliegende Randbins kragen aus und brauchen Stützen vom Druckbett".into(),
            Warning::PlateHeldByBins { sides } => format!(
                "Die offene Platte berührt die Schubladenwand {} nicht; die Randbins halten sie",
                sides.iter().map(|s| s.german()).collect::<Vec<_>>().join(" und ")
            ),
            Warning::PartTooLarge { part, size } => {
                format!("{part} ({:.1} × {:.1} mm) passt nicht aufs Druckbett", size[0], size[1])
            }
            Warning::ItemTooTall { item, units, max_units } => {
                format!("{item} braucht {units} Einheiten, die Schublade erlaubt {max_units}")
            }
            Warning::ItemUnplaced { item, cells } => {
                format!("Kein freier Platz von {}×{} Feldern für {item}", cells[0], cells[1])
            }
            Warning::HeightUnknown { units } => format!("Schubladenhöhe unbekannt, Bins bekommen {units} Einheiten"),
            Warning::WallHolderOffWall { holder } => {
                format!("{holder} ist ein Wandhalter; er liegt auf seiner Unterseite und gehört an eine Wand")
            }
            Warning::HolderNotForWall { holder } => format!(
                "{holder} ist für Schubladen gemacht: an der Wand öffnen sich die Taschen nach vorn, und die Haken der Platte lassen seine Füße nicht hinein"
            ),
            Warning::GridLargerThanSpace { axis, grid, space, wall } => format!(
                "Das Raster ist {grid:.0} mm {0}, die {1} nur {space:.0} mm: die Platte steht {2:.0} mm über",
                extent(*axis, *wall, true),
                if *wall { "Wand" } else { "Fläche" },
                grid - space
            ),
        }
    }

    /// In the given language ("de" or anything else for English).
    pub fn text(&self, language: &str) -> String {
        if language.starts_with("de") { self.german() } else { self.to_string() }
    }
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Warning::AlignForEdgeZone { axis, centred, one_side } => write!(
                f,
                "{} margins of {centred:.1} mm are too narrow to use; aligned to one wall, one margin of {one_side:.1} mm becomes usable",
                if *axis == 0 { "left/right" } else { "front/back" }
            ),
            Warning::SupportsNeeded => write!(f, "resting edge bins overhang and need supports from the build plate"),
            Warning::PlateHeldByBins { sides } => {
                write!(f, "the open plate does not touch the drawer wall on {sides:?}; the edge bins hold it")
            }
            Warning::PartTooLarge { part, size } => {
                write!(f, "{part} ({:.1} x {:.1} mm) does not fit the printer bed", size[0], size[1])
            }
            Warning::ItemTooTall { item, units, max_units } => {
                write!(f, "{item} needs {units} units, the drawer allows {max_units}")
            }
            Warning::ItemUnplaced { item, cells } => {
                write!(f, "no free {}x{} spot for {item}", cells[0], cells[1])
            }
            Warning::HeightUnknown { units } => write!(f, "drawer height unknown, bins get {units} units"),
            Warning::WallHolderOffWall { holder } => {
                write!(f, "{holder} is a wall holder; it lies on its bottom face and belongs on a wall")
            }
            Warning::HolderNotForWall { holder } => write!(
                f,
                "{holder} is made for drawers: on a wall its pockets open to the front and the plate's hooks keep its feet out"
            ),
            Warning::GridLargerThanSpace { axis, grid, space, wall } => write!(
                f,
                "the grid is {grid:.0} mm {0}, the {1} only {space:.0} mm: the plate sticks out by {2:.0} mm",
                extent(*axis, *wall, false),
                if *wall { "wall" } else { "surface" },
                grid - space
            ),
        }
    }
}
