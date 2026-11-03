//! Printer- and magnet-dependent settings. A profile is what a calibration piece measures.

use serde::{Deserialize, Serialize};

/// Magnet holes: in the baseplate under every cell, in bins at the corners or under every cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MagnetSpec {
    /// Hole diameter.
    pub d: f64,
    /// Hole depth.
    pub h: f64,
    /// Distance of the holes from the cell centre (Gridfinity: 26 mm between holes).
    pub pos: f64,
    /// Chamfer at the bin holes against elephant's foot (they sit on the build plate).
    pub chamfer: f64,
    /// Material around the holes in the baseplate floor.
    pub wall: f64,
    /// Floor under the holes of the baseplate.
    pub floor: f64,
}

impl MagnetSpec {
    /// 4 mm magnets, hole size found with the test rig (magnets sit about 0.2 mm below the surface).
    pub fn magnets_4mm() -> Self {
        MagnetSpec { d: 4.2, h: 1.4, pos: 13.0, chamfer: 0.4, wall: 1.6, floor: 0.8 }
    }

    /// The Gridfinity standard: 6 × 2 mm magnets.
    pub fn gridfinity_6x2() -> Self {
        MagnetSpec { d: 6.5, h: 2.4, ..Self::magnets_4mm() }
    }
}

impl Default for MagnetSpec {
    fn default() -> Self {
        Self::magnets_4mm()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub magnet: MagnetSpec,
    /// Segments of a full circle (rounded corners use a quarter of it).
    pub seg: u32,
    /// How much narrower a socket peg is than the drive square, so the socket slides on.
    pub peg_undersize: f64,
}

impl Default for Profile {
    fn default() -> Self {
        Profile { name: "4-mm-Magnete".into(), magnet: MagnetSpec::magnets_4mm(), seg: 64, peg_undersize: 0.3 }
    }
}

impl Profile {
    pub fn builtin() -> Vec<Profile> {
        vec![
            Profile::default(),
            Profile { name: "Gridfinity 6×2".into(), magnet: MagnetSpec::gridfinity_6x2(), ..Profile::default() },
        ]
    }
}
