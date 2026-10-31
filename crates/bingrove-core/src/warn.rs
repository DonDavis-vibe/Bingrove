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
