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
        ..TroughSpec::default()
    });
    let part = holders::holder(&sd, &p);
    assert!(part.mesh().unwrap().is_closed(), "{}", part.name);
    assert!(capacity(&sd) >= 2);
    assert!(part.name.starts_with("halter_schraubendreher_d8_4x2_"));
}

#[test]
fn rows_slots_and_shapes() {
    let p = Profile::default();
    // A metric 1/4" socket row: one strip, thirteen holes of growing size, a label strip in front.
    let set = bingrove_core::catalog::sets().into_iter().find(|s| s.id == "sockets_1_4_mm").unwrap().holder;
    let part = holders::holder(&set, &p);
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed(), "{}", part.name);
    assert!(part.name.starts_with("halter_reihe_1_4_Zoll_"), "{}", part.name);
    let (lo, hi) = mesh.bbox();
    let size = set.units() as f64 * UNIT + LIP_H;
    assert!(((hi[2] - lo[2]) as f64 - size).abs() < 1e-3);
    let cells = set.cells();
    let shell = plain(cells, set.units(), &p);
    let [w, d] = cells.map(bin_outer);
    let insert_full = (w - 2.4) * (d - 2.4) * (size - LIP_H - 0.8 - 5.95);
    let v = mesh.volume();
    assert!(v > shell && v < shell + insert_full, "{v} vs shell {shell}");
    // Holes take out at least the sum of their cylinders' lower halves.
    let HolderSpec::Row(r) = &set else { panic!() };
    let holes: f64 = r.rows[0].iter().map(|h| std::f64::consts::PI * (h.d / 2.0).powi(2) * h.depth() * 0.5).sum();
    assert!(shell + insert_full - v > holes, "holes missing: {}", shell + insert_full - v);

    // Two stepped rows stand a unit taller than one, and the back strip is higher than the front.
    let two = bingrove_core::catalog::sets().into_iter().find(|s| s.id == "sockets_1_4_both").unwrap().holder;
    assert_eq!(two.units(), set.units() + 1);
    let part = holders::holder(&two, &p);
    assert!(part.mesh().unwrap().is_closed(), "{}", part.name);
    assert!(part.name.contains("_gestuft_"), "{}", part.name);

    // Slots for swatches: thirteen in a 2 x 1 holder.
    let swatches = catalog().into_iter().find(|c| c.id == "swatches").unwrap().holder;
    let part = holders::holder(&swatches, &p);
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed());
    assert_eq!(part.name, "halter_schlitze_3_2x1_5u.stl");
    let shell = plain([2, 1], swatches.units(), &p);
    let [w, d] = [2, 1].map(bin_outer);
    let insert_full = (w - 2.4) * (d - 2.4) * (swatches.size()[2] - LIP_H - 0.8 - 5.95);
    let v = mesh.volume();
    let slot_volume = 13.0 * 3.0 * (d - 2.4 - 1.6) * 24.0;
    assert!(v < shell + insert_full - slot_volume * 0.8 && v > shell, "{v}");

    // Hex shanks: a hex hole holder is watertight and named for its shape.
    let hex = catalog().into_iter().find(|c| c.id == "hex_3_8").unwrap().holder;
    let part = holders::holder(&hex, &p);
    assert!(part.mesh().unwrap().is_closed(), "{}", part.name);
    assert!(part.name.starts_with("halter_loch_d9.53_hex_"), "{}", part.name);
}

#[test]
fn pegs_and_cradles() {
    use bingrove_core::catalog::sets;
    use bingrove_core::spec::{PEG_BASE, PEG_H};
    let p = Profile::default();
    // The 1/4" socket set on pegs: same spacing, pegs stand on a thin base instead of holes.
    let set = sets().into_iter().find(|s| s.id == "pegs_1_4_mm").unwrap().holder;
    let HolderSpec::Row(r) = &set else { panic!() };
    assert_eq!(r.pegs, Some(bingrove_core::model::Drive::Quarter));
    assert_eq!(set.units(), ((bingrove_core::spec::FLOOR_Z + PEG_BASE + PEG_H + 2.0) / UNIT).ceil() as u32);
    let part = holders::holder(&set, &p);
    assert!(part.name.contains("_stecker14_"), "{}", part.name);
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed());
    // The pegs add material to the base: more volume than the same shell with a bare base.
    let holes = sets().into_iter().find(|s| s.id == "sockets_1_4_mm").unwrap().holder;
    let plain_base = holders::holder(
        &HolderSpec::Row(bingrove_core::model::RowSpec {
            rows: vec![vec![]],
            pegs: Some(bingrove_core::model::Drive::Quarter),
            cells: r.cells,
            units: Some(set.units()),
            ..bingrove_core::model::RowSpec::default()
        }),
        &p,
    )
    .mesh()
    .unwrap()
    .volume();
    let peg = (6.35_f64 - p.peg_undersize).powi(2) * PEG_H;
    assert!(mesh.volume() > plain_base + 13.0 * peg * 0.8, "pegs missing: {} vs {plain_base}", mesh.volume());
    assert_ne!(holes.cells(), [0, 0]);

    // Cradles: three stepped beds, the right one a unit higher, every bed rising to the back.
    let ratchets = sets().into_iter().find(|s| s.id == "ratchets").unwrap().holder;
    let part = holders::holder(&ratchets, &p);
    assert!(part.name.starts_with("halter_wiegen_Ratschen_gestuft_"), "{}", part.name);
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed(), "{}", part.name);
    let (lo, hi) = mesh.bbox();
    let size = ratchets.size();
    assert!(((hi[2] - lo[2]) as f64 - size[2]).abs() < 1e-3);
    assert!(lo.iter().all(|v| v.abs() < 1e-3));
    let HolderSpec::Cradles(c) = &ratchets else { panic!() };
    assert!(c.cells[0] >= 3 && c.cells[1] >= 6, "{:?}", c.cells);
    let shell = plain(c.cells, ratchets.units(), &p);
    assert!(mesh.volume() > shell, "the wedges fill the bin");
}

#[test]
fn vslots_tilted_rows_and_rings() {
    use bingrove_core::catalog::sets;
    let p = Profile::default();
    let find = |id: &str| sets().into_iter().find(|s| s.id == id).unwrap().holder;
    // Wrenches on a ramp: watertight, named for the ramp, slots take material out.
    let wrenches = find("wrenches");
    let part = holders::holder(&wrenches, &p);
    assert!(part.name.starts_with("halter_vschlitze_Maulschluessel_rampe_"), "{}", part.name);
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed(), "{}", part.name);
    let HolderSpec::VSlots(v) = &wrenches else { panic!() };
    let shell = plain(v.cells, wrenches.units(), &p);
    let [w, d] = v.cells.map(bin_outer);
    let insert_full = (w - 2.4) * (d - 2.4) * (wrenches.size()[2] - LIP_H - 0.8 - 5.95);
    assert!(mesh.volume() > shell && mesh.volume() < shell + insert_full * 0.9, "{}", mesh.volume());
    // The slots really go through the insert: at least half of each slot's box comes out.
    let top = (5.95 + 1.2 + v.max_depth()).min(wrenches.size()[2] - LIP_H - 0.8);
    let insert = (w - 2.4) * (d - 2.4) * (top - 5.95);
    let slots: f64 = v.slots.iter().map(|s| s.width * s.depth.min(top - 7.15 - 14.0) / 2.0 * (d - 10.0)).sum();
    let removed = shell + insert - mesh.volume();
    assert!(removed > slots, "removed {removed:.0} mm³, slots {slots:.0} mm³");
    // Nothing reaches through the floor: the solid is closed under the insert.
    let probe = bingrove_core::geom::kernel::cuboid([w - 10.0, d - 10.0, 0.9], [5.0, 5.0, 5.0]);
    let floor = &part.solid ^ &probe;
    let floor_v = (w - 10.0) * (d - 10.0) * 0.9;
    assert!((floor.volume() - floor_v).abs() < floor_v * 0.01, "floor {:.0} of {floor_v:.0}", floor.volume());
    // Pliers without a ramp, open at the front: still watertight.
    let pliers = holders::holder(&find("pliers"), &p);
    assert!(pliers.mesh().unwrap().is_closed());
    // Tilted hex keys: the holes lean back, the mouths stay where upright ones would be.
    let tilted = find("hex_keys_tilted");
    let part = holders::holder(&tilted, &p);
    assert!(part.name.contains("_schraeg_"), "{}", part.name);
    assert!(part.mesh().unwrap().is_closed());
    let HolderSpec::Row(r) = &tilted else { panic!() };
    let upright = HolderSpec::Row(bingrove_core::model::RowSpec { tilt: 0.0, ..r.clone() });
    let (vt, vu) = (part.mesh().unwrap().volume(), holders::holder(&upright, &p).mesh().unwrap().volume());
    assert!((vt - vu).abs() < vu * 0.05, "tilting keeps the holes' volume: {vt} vs {vu}");
    // Rings: a 2 x 2 holder takes one spray can, the shell is as low as the rings.
    let cans = find("spray_cans");
    assert_eq!(bingrove_core::catalog::capacity(&cans), 1);
    let part = holders::holder(&cans, &p);
    assert_eq!(part.name, "halter_ringe_d66_2x2_6u.stl", "30 mm rings above the floor need six units");
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed());
    let HolderSpec::Rings(rg) = &cans else { panic!() };
    assert!(mesh.volume() > plain(rg.cells, cans.units(), &p), "the ring adds material");
}

#[test]
fn engraving_takes_material_and_names_the_part() {
    let p = Profile::default();
    let plain_bin = BinSpec { cells: [2, 1], units: 6, label_tab: true, ..BinSpec::default() };
    let v0 = bin::bin(&plain_bin, &p).mesh().unwrap().volume();
    let shelf = BinSpec { label_text: Some("M3 x 12".into()), ..plain_bin.clone() };
    let part = bin::bin(&shelf, &p);
    assert_eq!(part.name, "bin_2x1_6u_label_M3_x_12.stl");
    let mesh = part.mesh().unwrap();
    assert!(mesh.is_closed());
    let v1 = mesh.volume();
    assert!(v1 < v0 - 5.0 && v1 > v0 - 60.0, "shelf engraving: {v0} -> {v1}");
    // Without a shelf the text goes into the front wall.
    let front = BinSpec { label_tab: false, ..shelf.clone() };
    let v2 = bin::bin(&front, &p).mesh().unwrap().volume();
    let v_plain = bin::bin(&BinSpec { label_tab: false, ..plain_bin.clone() }, &p).mesh().unwrap().volume();
    assert!(v2 < v_plain - 3.0 && v2 > v_plain - 40.0, "front engraving: {v_plain} -> {v2}");
    // A row holder with a title is engraved on its front too and stays watertight.
    let mut set = bingrove_core::catalog::sets().into_iter().find(|s| s.id == "drills").unwrap().holder;
    let labelled = holders::holder(&set, &p).mesh().unwrap();
    assert!(labelled.is_closed());
    if let HolderSpec::Row(r) = &mut set {
        r.title = None;
    }
    let untitled = holders::holder(&set, &p).mesh().unwrap().volume();
    assert!(labelled.volume() < untitled, "the title is cut in: {} vs {untitled}", labelled.volume());
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
    assert_eq!(parts.len(), 15);
    assert_eq!(parts[7].name, "kalibrierung_stecker.stl");
    let wall: Vec<_> = parts[8..].iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        wall,
        [
            "kalibrierung_wand_zelle.stl",
            "kalibrierung_wand_haken_0.2.stl",
            "kalibrierung_wand_haken_0.3.stl",
            "kalibrierung_wand_haken_0.4.stl",
            "kalibrierung_wand_raste_0.3.stl",
            "kalibrierung_wand_raste_0.5.stl",
            "kalibrierung_wand_raste_0.7.stl"
        ]
    );
    let names: Vec<_> = parts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names[0], "kalibrierung_magnete_3.9-4.5.stl");
    assert_eq!(
        names[2..5],
        ["kalibrierung_passung_bin_0.15.stl", "kalibrierung_passung_bin_0.25.stl", "kalibrierung_passung_bin_0.35.stl"]
    );
    assert_eq!(names[5..7], ["kalibrierung_verbinder_a.stl", "kalibrierung_verbinder_b.stl"]);
    // The tab fits its pocket: a laid against b (b's left edge at a's right edge) leaves no overlap.
    let (a, b) = (&parts[5].solid, &parts[6].solid);
    let joined = (a ^ &b.translate(bingrove_core::geom::kernel::Vec3::new(24.0, 0.0, 0.0))).volume();
    assert!(joined < 1e-3, "tab and pocket overlap by {joined}");
    for part in &parts {
        let mesh = part.mesh().unwrap();
        assert!(mesh.is_closed(), "{}", part.name);
    }
    let (lo, hi) = parts[2].mesh().unwrap().bbox();
    assert!(((hi[0] - lo[0]) as f64 - 41.7).abs() < 1e-3, "0.15 mm air each side");
}
