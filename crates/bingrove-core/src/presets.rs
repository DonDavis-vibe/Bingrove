//! The drawers of the original Python script, kept as JSON so they double as examples of the format.

use serde::{Deserialize, Serialize};

use crate::model::{BinSpec, Placement, PlateSpec, TapeSpec};

const SOURCES: [&str; 2] = [include_str!("../presets/435x330.json"), include_str!("../presets/250x305.json")];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preset {
    pub name: String,
    pub description: String,
    pub plate: PlateSpec,
    /// Height of single bins made for this drawer.
    pub units: u32,
    /// A fixed layout that fills the grid exactly ...
    pub placements: Vec<Placement>,
    /// ... or an assortment of bins to print in any number.
    pub bins: Vec<BinSpec>,
    pub tape: Vec<TapeSpec>,
}

impl Default for Preset {
    fn default() -> Self {
        Preset {
            name: String::new(),
            description: String::new(),
            plate: PlateSpec::default(),
            units: 6,
            placements: vec![],
            bins: vec![],
            tape: vec![],
        }
    }
}

impl Preset {
    pub fn all() -> Vec<Preset> {
        SOURCES.iter().map(|s| serde_json::from_str(s).expect("built-in preset")).collect()
    }

    pub fn get(name: &str) -> Option<Preset> {
        Self::all().into_iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_presets_parse() {
        let names: Vec<_> = Preset::all().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["435x330", "250x305"]);
        let big = Preset::get("435x330").unwrap();
        assert_eq!(big.plate.cells(), [10, 7]);
        assert_eq!(big.bins.len(), 6);
        assert!(big.bins.iter().all(|b| b.units == 9 && b.lip));
        assert_eq!(big.tape.len(), 4);
    }
}
