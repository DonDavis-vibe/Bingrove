//! Holders, cutouts and calibration pieces: watertight, the right size, and the pockets really
//! take material away.

use bingrove_core::catalog::{capacity, catalog};
use bingrove_core::geom::{bin, calibration, holders};
use bingrove_core::model::{BinSpec, Cutout, EdgeMode, HolderSpec, HolesSpec, RoundItem, TroughSpec};
use bingrove_core::profile::Profile;
use bingrove_core::spec::{LIP_H, UNIT, bin_outer};

fn plain(cells: [u32; 2], units: u32, p: &Profile) -> f64 {
    bin::bin(&BinSpec { cells, units, ..BinSpec::default() }, p).mesh().unwrap().volume()
}

#[test]
fn every_catalog_holder_is_watertight_and_sized() {
    let p = Profile::default();
    for entry in catalog() {
        let part = holders::holder(&entry.holder, &p);
        let mesh = part.mesh().unwrap();
        assert!(mesh.is_closed(), "{} ({}) is not watertight", entry.id, part.name);
        let (lo, hi) = mesh.bbox();
        let size = entry.holder.size();
        for k in 0..3 {
            assert!(lo[k].abs() < 1e-3, "{}: min {lo:?}", entry.id);
            assert!(
                ((hi[k] - lo[k]) as f64 - size[k]).abs() < 0.05,
                "{}: {:?} vs {size:?}",
                entry.id,
                [0, 1, 2].map(|k| hi[k] - lo[k])
            );
        }
        assert!(capacity(&entry.holder) >= 1, "{}", entry.id);
    }
}

#[test]
fn battery_holder_has_its_holes() {
    let p = Profile::default();
    let spec = HolesSpec { item: RoundItem::Cell18650, cells: [2, 1], ..HolesSpec::default() };
    let holder = HolderSpec::Holes(spec.clone());
    let n = capacity(&holder);
    assert_eq!(n, 3, "three 18650 holes at 20.8 mm pitch along 79.5 mm, one across");
    let part = holders::holder(&holder, &p);
    assert_eq!(part.name, "halter_akku_18650_2x1_7u.stl");
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed());
    // The insert fills the bin; the holes take out n cylinders of about 19.2 mm diameter.
    let units = holder.units();
    let shell = plain([2, 1], units, &p);
    let [w, d] = [2, 1].map(bin_outer);
    let insert_full = (w - 2.4) * (d - 2.4) * (holder.size()[2] - LIP_H - 0.8 - 5.95);
    let v = mesh.volume();
    assert!(v > shell && v < shell + insert_full, "{v} vs shell {shell}");
    let hole = std::f64::consts::PI * 9.6 * 9.6 * (65.0 * 0.55);
    assert!(shell + insert_full - v > n as f64 * hole * 0.8, "holes missing: {}", shell + insert_full - v);
    assert!(((holder.size()[2] - LIP_H) / UNIT - units as f64).abs() < 1e-9);
}

#[test]
fn bits_and_troughs() {
    let p = Profile::default();
    let bits = holders::holder(&HolderSpec::Bits(Default::default()), &p);
    assert!(bits.mesh().unwrap().is_closed());
    assert_eq!(bits.name, "halter_bits_1x1_3u.stl");
    let sd = HolderSpec::Troughs(TroughSpec {
        diameter: 8.0,
        length: 200.0,
        handle: Some(30.0),
        cells: [4, 2],
        units: None,
    });
    let part = holders::holder(&sd, &p);
    assert!(part.mesh().unwrap().is_closed(), "{}", part.name);
    assert!(capacity(&sd) >= 2);
    assert!(part.name.starts_with("halter_schraubendreher_d8_4x2_"));
}

#[test]
fn cutouts_pocket_the_insert() {
    let p = Profile::default();
    let spec = BinSpec {
        cells: [2, 2],
        units: 6,
        cutouts: vec![
            Cutout::Box { at: [25.0, 25.0], size: [30.0, 20.0], depth: 15.0 },
            Cutout::Cylinder { at: [60.0, 55.0], d: 18.0, depth: 10.0 },
            Cutout::Polygon { points: vec![[10.0, 55.0], [35.0, 55.0], [22.0, 78.0]], clearance: 1.0, depth: 8.0 },
        ],
        ..BinSpec::default()
    };
    let part = bin::edge_bin(&spec, [0.0; 4], EdgeMode::default(), 1.0, &p);
    assert_eq!(part.name, "bin_2x2_6u_3ausschnitt.stl");
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed());
    let v = mesh.volume();
    let shell = plain([2, 2], 6, &p);
    // Insert up to 15 + 1.2 above the floor, minus the three pockets.
    let [w, d] = [2, 2].map(bin_outer);
    let insert = (w - 2.4) * (d - 2.4) * 16.2;
    let pockets = 30.0 * 20.0 * 15.0 + std::f64::consts::PI * 81.0 * 10.0 + 0.5 * 25.0 * 23.0 * 8.0;
    assert!((v - (shell + insert - pockets)).abs() < 0.12 * insert, "{v} vs {}", shell + insert - pockets);
    let (_, hi) = mesh.bbox();
    assert!((hi[2] as f64 - (6.0 * UNIT + LIP_H)).abs() < 1e-3);
}

#[test]
fn calibration_pieces() {
    let p = Profile::default();
    let parts = calibration::parts(&p);
    assert_eq!(parts.len(), 5);
    let names: Vec<_> = parts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names[0], "kalibrierung_magnete_3.9-4.5.stl");
    assert_eq!(
        names[2..],
        ["kalibrierung_passung_bin_0.15.stl", "kalibrierung_passung_bin_0.25.stl", "kalibrierung_passung_bin_0.35.stl"]
    );
    for part in &parts {
        let mesh = part.mesh().unwrap();
        assert!(mesh.is_closed(), "{}", part.name);
    }
    let (lo, hi) = parts[2].mesh().unwrap().bbox();
    assert!(((hi[0] - lo[0]) as f64 - 41.7).abs() < 1e-3, "0.15 mm air each side");
}
