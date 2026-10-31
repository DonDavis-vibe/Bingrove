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
        }
    }
}
