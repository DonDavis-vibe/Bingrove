//! Text on lids: engraved into the plate inside the lip and filled by a second body, flush, to
//! print in another colour; how plans pick the text and how it is exported.

use std::io::Read;

use bingrove_core::export::{ExportOptions, FileKind, export};
use bingrove_core::geom::kernel::cuboid;
use bingrove_core::geom::lid;
use bingrove_core::model::{BinSpec, DrawerProject, DrawerSpec, LidSpec, Placement};
use bingrove_core::plan::plan;
use bingrove_core::profile::Profile;
use bingrove_core::spec::{LID_PLUG, LID_TEXT_DEPTH, bin_outer};

fn labelled(text: &str) -> LidSpec {
    LidSpec { cells: [2, 1], label: Some(text.into()), ..LidSpec::default() }
}

#[test]
fn a_lid_text_is_engraved_and_filled_flush() {
    let p = Profile::default();
    let plain = lid::lid(&LidSpec { cells: [2, 1], ..LidSpec::default() }, &p);
    let part = lid::lid(&labelled("Schrauben M4"), &p);
    assert_eq!(part.name, "deckel_2x1_klick_Schrauben_M4.stl");
    assert!(plain.inlay.is_none(), "a lid without text has no inlay");
    let inlay = part.inlay.as_ref().expect("a lid with text has an inlay");
    assert!(part.mesh().unwrap().is_closed() && part.inlay_mesh().unwrap().unwrap().is_closed());
    // The inlay fills exactly what the engraving took out, and the two do not overlap.
    let taken = plain.solid.volume() - part.solid.volume();
    assert!(taken > 10.0, "the engraving takes only {taken:.2} mm³");
    assert!((inlay.volume() - taken).abs() < 0.01 * taken, "inlay {:.2} mm³, engraving {taken:.2} mm³", inlay.volume());
    assert!((&part.solid ^ inlay).volume() < 0.01);
    // Flush with the plate's top, LID_TEXT_DEPTH deep, inside the lip.
    let plate_top = LID_PLUG + LidSpec::default().thickness;
    let b = inlay.bounding_box();
    assert!((b.max.z - plate_top).abs() < 1e-6 && (b.min.z - (plate_top - LID_TEXT_DEPTH)).abs() < 1e-6, "{b:?}");
    let (w, d) = (bin_outer(2), bin_outer(1));
    assert!(b.min.x > 2.6 && b.max.x < w - 2.6 && b.min.y > 2.6 && b.max.y < d - 2.6, "{b:?}");
    // Read from the front: wider than tall, centred.
    assert!(b.max.x - b.min.x > b.max.y - b.min.y);
    assert!(((b.min.x + b.max.x) / 2.0 - w / 2.0).abs() < 2.0 && ((b.min.y + b.max.y) / 2.0 - d / 2.0).abs() < 2.0);
    // Nothing above the plate in the text's area: a bin stood on the lid rests on it as before.
    let above = cuboid([w - 6.0, d - 6.0, 0.5], [3.0, 3.0, plate_top]);
    assert!((&part.solid ^ &above).volume() < 1e-6 && (inlay ^ &above).volume() < 1e-6);
}

#[test]
fn a_long_text_shrinks_to_fit_and_an_empty_one_is_none() {
    let p = Profile::default();
    let part =
        lid::lid(&LidSpec { cells: [1, 1], label: Some("Elektronik-Kleinteile".into()), ..LidSpec::default() }, &p);
    let b = part.inlay.as_ref().unwrap().bounding_box();
    assert!(b.min.x > 2.6 && b.max.x < bin_outer(1) - 2.6, "{b:?}");
    let blank = lid::lid(&LidSpec { label: Some("   ".into()), ..LidSpec::default() }, &p);
    assert!(blank.inlay.is_none() && blank.name == "deckel_1x1_klick.stl");
}

fn project(placements: Vec<Placement>) -> DrawerProject {
    DrawerProject {
        drawer: DrawerSpec { name: "Ablage".into(), width: 180.0, depth: 108.0, height: 60.0 },
        placements,
        ..DrawerProject::default()
    }
}

fn lidded(x: u32, y: u32, engraving: Option<&str>, lid_label: Option<&str>) -> Placement {
    let bin = BinSpec { cells: [2, 1], units: 6, label_text: engraving.map(Into::into), ..BinSpec::default() };
    Placement { lid: true, lid_label: lid_label.map(Into::into), ..Placement::bin(x, y, bin) }
}

#[test]
fn a_lid_takes_its_own_text_else_the_engraving() {
    let plan_ = plan(&project(vec![
        lidded(0, 0, Some("Kabel"), None),
        lidded(0, 1, Some("Kabel"), Some("  ")),
        lidded(2, 0, Some("Kabel"), Some("Akkus")),
        Placement { lid: true, ..Placement::bin(2, 1, BinSpec { cells: [1, 1], units: 6, ..BinSpec::default() }) },
    ]))
    .unwrap();
    let mut lids: Vec<_> =
        plan_.parts.iter().filter(|b| b.name.starts_with("deckel_")).map(|b| (b.name.as_str(), b.quantity)).collect();
    lids.sort();
    assert_eq!(
        lids,
        [("deckel_1x1_klick.stl", 1), ("deckel_2x1_klick_Akkus.stl", 1), ("deckel_2x1_klick_Kabel.stl", 2)]
    );
}

#[test]
fn a_lid_with_text_exports_its_inlay_beside_it_and_as_a_part_on_the_bed() {
    let project = project(vec![lidded(0, 0, None, Some("Kabel")), lidded(0, 1, None, None)]);
    let plan = plan(&project).unwrap();
    let out = std::env::temp_dir().join(format!("bingrove-lids-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let report = export(&plan, &project, &out, ExportOptions::default(), &|_, _, _| {}).unwrap();
    let inlays: Vec<_> = report.files.iter().filter(|f| f.kind == FileKind::Inlay).map(|f| f.path.as_str()).collect();
    assert_eq!(inlays, ["deckel_2x1_klick_Kabel_schrift.stl"]);
    assert!(out.join(inlays[0]).exists());
    // Not a part of its own: the parts list (and so the print list) does not have it.
    assert!(report.files.iter().filter(|f| f.kind == FileKind::Part).all(|f| !f.path.ends_with("_schrift.stl")));
    // On the bed with the lids: one object of two parts, the text on the second filament.
    let bed = report
        .files
        .iter()
        .filter(|f| f.kind == FileKind::Bed)
        .map(|f| out.join(&f.path))
        .find(|path| {
            let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
            zip.by_name("Metadata/model_settings.config").is_ok()
        })
        .expect("a bed with the labelled lid");
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&bed).unwrap()).unwrap();
    let mut model = String::new();
    zip.by_name("3D/3dmodel.model").unwrap().read_to_string(&mut model).unwrap();
    assert_eq!(model.matches("<components>").count(), 1, "one lid with text");
    assert!(model.contains("deckel_2x1_klick_Kabel.stl – Schrift"));
    let mut settings = String::new();
    zip.by_name("Metadata/model_settings.config").unwrap().read_to_string(&mut settings).unwrap();
    assert!(settings.contains("value=\"Schrift\"") && settings.contains("key=\"extruder\" value=\"2\""), "{settings}");
    let _ = std::fs::remove_dir_all(&out);
}
