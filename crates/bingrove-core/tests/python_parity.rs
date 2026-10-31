//! The port must reproduce the parts of `gridfinity_schublade.py`. Reference bounding boxes and
//! volumes were measured from the STL files the script wrote (`stl/` and `stl_250x305/`).

use bingrove_core::geom::{Part, baseplate, bin, tape};
use bingrove_core::model::{BinSpec, PlateSpec};
use bingrove_core::presets::Preset;
use bingrove_core::profile::Profile;

/// (file, size x/y/z, volume in mm³)
const REFERENCE: &[(&str, [f32; 3], f64)] = &[
    ("baseplate_1A_5x4.stl", [217.0, 185.5, 4.65], 39371.8),
    ("baseplate_1B_5x4.stl", [217.0, 185.5, 4.65], 39371.8),
    ("baseplate_2A_5x3.stl", [217.0, 143.5, 4.65], 31546.7),
    ("baseplate_2B_5x3.stl", [217.0, 143.5, 4.65], 31546.7),
    ("bin_1x1_9u.stl", [41.5, 41.5, 67.4], 20772.0),
    ("bin_1x2_9u.stl", [41.5, 83.5, 67.4], 36132.3),
    ("bin_1x3_9u.stl", [41.5, 125.5, 67.4], 51492.5),
    ("bin_2x1_9u_2x1fach.stl", [83.5, 41.5, 67.4], 38771.1),
    ("bin_2x2_9u.stl", [83.5, 83.5, 67.4], 60473.7),
    ("bin_2x3_9u.stl", [83.5, 125.5, 67.4], 84815.2),
    ("tape_3x3_1platz_2hoch_bis124mm.stl", [125.5, 125.5, 67.94], 123590.7),
    ("tape_3x3_1platz_3hoch_bis124mm.stl", [125.5, 125.5, 117.94], 156564.2),
    ("tape_5x3_2plaetze_2hoch_bis103mm.stl", [209.5, 125.5, 67.94], 220352.8),
    ("tape_5x3_2plaetze_3hoch_bis103mm.stl", [209.5, 125.5, 117.94], 286299.7),
    ("baseplate_1A_3x4.stl", [145.5, 173.0, 4.65], 25371.8),
    ("baseplate_1B_2x4.stl", [103.5, 173.0, 4.65], 19228.9),
    ("baseplate_2A_3x3.stl", [145.5, 131.0, 4.65], 19870.6),
    ("baseplate_2B_2x3.stl", [103.5, 131.0, 4.65], 15019.2),
    ("bin_1x1_6u.stl", [41.5, 41.5, 46.4], 16846.9),
    ("bin_1x2_12u.stl", [41.5, 83.5, 88.4], 42174.2),
    ("bin_1x5_6u.stl", [41.5, 209.5, 46.4], 69820.6),
    ("bin_2x1_6u_2x1fach.stl", [83.5, 41.5, 46.4], 31743.8),
    ("bin_2x2_12u.stl", [83.5, 83.5, 88.4], 68632.5),
    ("bin_2x2_6u.stl", [83.5, 83.5, 46.4], 52315.0),
    ("bin_2x3_6u.stl", [83.5, 125.5, 46.4], 74539.6),
];

fn check(part: &Part) {
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed(), "{} is not watertight", part.name);
    let (lo, hi) = mesh.bbox();
    let (_, size, volume) =
        REFERENCE.iter().find(|(n, ..)| *n == part.name).unwrap_or_else(|| panic!("no reference for {}", part.name));
    for k in 0..3 {
        assert!(lo[k].abs() < 0.01, "{}: minimum corner {lo:?}", part.name);
        assert!(
            (hi[k] - lo[k] - size[k]).abs() < 0.01,
            "{}: size {:?} vs {size:?}",
            part.name,
            [0, 1, 2].map(|k| hi[k] - lo[k])
        );
    }
    let v = mesh.volume();
    assert!((v - volume).abs() / volume < 0.005, "{}: volume {v:.1} vs {volume}", part.name);
}

fn plate_of(p: &Preset) -> PlateSpec {
    p.plate.clone()
}

#[test]
fn baseplates_match() {
    let profile = Profile::default();
    for preset in Preset::all() {
        for part in baseplate::pieces(&plate_of(&preset), &profile) {
            check(&part);
        }
    }
}

#[test]
fn bins_match() {
    let profile = Profile::default();
    let big = Preset::get("435x330").unwrap();
    for b in &big.bins {
        check(&bin::bin(b, &profile));
    }
    let small = Preset::get("250x305").unwrap();
    for (b, _) in bingrove_core::layout::bill_of_bins(&small.placements) {
        check(&bin::bin(&b, &profile));
    }
}

#[test]
fn tape_holders_match() {
    let profile = Profile::default();
    for t in &Preset::get("435x330").unwrap().tape {
        check(&tape::tape(t, &profile));
    }
}

#[test]
fn magnet_variants_are_watertight() {
    let profile = Profile::default();
    let plate = PlateSpec { magnets: true, ..Preset::get("250x305").unwrap().plate };
    let parts = baseplate::pieces(&plate, &profile);
    let (lo, hi) = parts[0].mesh().unwrap().bbox();
    assert!(((hi[2] - lo[2]) as f64 - (4.65 + 1.4 + 0.8)).abs() < 1e-3);
    for part in &parts {
        assert!(part.mesh().unwrap().is_closed(), "{}", part.name);
    }
    for magnets in [bingrove_core::model::MagnetMode::Corners, bingrove_core::model::MagnetMode::All] {
        let b = BinSpec { cells: [2, 1], magnets, ..BinSpec::default() };
        let part = bin::bin(&b, &profile);
        let plain = bin::bin(&BinSpec { magnets: Default::default(), ..b.clone() }, &profile);
        let holes = if magnets == bingrove_core::model::MagnetMode::All { 8.0 } else { 4.0 };
        let removed = plain.mesh().unwrap().volume() - part.mesh().unwrap().volume();
        let one = std::f64::consts::PI * 2.1 * 2.1 * 1.4;
        assert!(part.mesh().unwrap().is_closed());
        assert!(removed > holes * one * 0.95 && removed < holes * one * 1.3, "{magnets:?}: {removed}");
    }
}
