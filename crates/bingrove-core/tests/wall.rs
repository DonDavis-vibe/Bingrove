//! Wall plates and the parts that hang on them: the hook lets a part in when it is lifted, keeps
//! it from being pulled straight out when it hangs, and the assembly stands up.

use bingrove_core::geom::kernel::{Solid, Vec3};
use bingrove_core::geom::{all_parts, assemble, wall};
use bingrove_core::model::{BinSpec, DrawerProject, DrawerSpec, Kind, MagnetMode, Placement, PlateFloor, PlateOptions};
use bingrove_core::plan::{PartSpec, Plan, plan};
use bingrove_core::profile::Profile;
use bingrove_core::spec::{BIN_GAP, HOOK_LIFT, UNIT, wall_size};

fn wall_project(hooks: bool, placements: Vec<Placement>) -> DrawerProject {
    DrawerProject {
        drawer: DrawerSpec { name: "Werkstattwand".into(), width: 3.0 * 42.0, depth: 3.0 * 42.0, height: 60.0 },
        options: PlateOptions { kind: Kind::Wall, floor: PlateFloor::Magnets, hooks, ..PlateOptions::default() },
        placements,
        ..DrawerProject::default()
    }
}

fn tray(x: u32, y: u32, cells: [u32; 2]) -> Placement {
    Placement::bin(x, y, BinSpec { cells, units: 6, magnets: MagnetMode::Corners, ..BinSpec::default() })
}

/// The plate in plate coordinates and the first placed part moved from its print frame onto the
/// plate, seated, plus where it sits.
fn seated(plan: &Plan, p: &Profile) -> (Solid, Solid) {
    let parts = all_parts(plan, p);
    let plate = parts
        .iter()
        .zip(&plan.parts)
        .filter(|(_, e)| matches!(e.part, PartSpec::Plate { .. }))
        .map(|(part, _)| part.solid.translate(Vec3::new(part.origin[0], part.origin[1], 0.0)))
        .reduce(|a, b| a + b)
        .unwrap();
    let placed = &plan.placements[0];
    let depth = plan.parts[placed.part].size[1];
    let [x, y, _] = plan.position(placed);
    // Print (x, y, z) is plate (x, z, depth − y), resting on the plate floor.
    let part = parts[placed.part].solid.rotate(-90.0, 0.0, 0.0).translate(Vec3::new(x, y, depth + plan.grid.floor));
    (plate, part)
}

fn overlap(a: &Solid, b: &Solid, dy: f64, dz: f64) -> f64 {
    (a ^ &b.translate(Vec3::new(0.0, dy, dz))).volume()
}

#[test]
fn a_part_goes_in_lifted_slides_down_and_hangs() {
    let p = Profile { seg: 32, ..Profile::default() };
    let plan = plan(&wall_project(true, vec![tray(1, 1, [1, 1])])).unwrap();
    let (plate, part) = seated(&plan, &p);
    // Seated, and settled down onto its lower foot faces: no overlap.
    assert!(overlap(&plate, &part, 0.0, 0.0) < 0.01, "seated: {}", overlap(&plate, &part, 0.0, 0.0));
    let settled = -BIN_GAP + 0.02;
    assert!(overlap(&plate, &part, settled, 0.0) < 0.01, "settled: {}", overlap(&plate, &part, settled, 0.0));
    // Lifted, it goes straight in past the hook ...
    for dz in [8.0, 6.0, 4.0, 2.5, 1.0, 0.0] {
        let v = overlap(&plate, &part, HOOK_LIFT, dz);
        assert!(v < 0.01, "going in lifted, {dz} mm out: {v} mm³");
    }
    // ... and slides down onto its seat.
    for k in 0..=12 {
        let dy = HOOK_LIFT - k as f64 * (HOOK_LIFT - settled) / 12.0;
        let v = overlap(&plate, &part, dy, 0.0);
        assert!(v < 0.01, "sliding down at {dy:.2} mm: {v} mm³");
    }
    // Hanging, pulling it straight out runs the feet into the hooks' fingers.
    let pulled = overlap(&plate, &part, 0.0, 1.0);
    assert!(pulled > 3.0, "pulled out 1 mm: only {pulled} mm³ in the way");
    let pulled = overlap(&plate, &part, settled, 1.0);
    assert!(pulled > 3.0, "settled and pulled out 1 mm: only {pulled} mm³ in the way");
    // Lifted again, it comes out.
    assert!(overlap(&plate, &part, HOOK_LIFT, 1.0) < 0.01);
}

#[test]
fn without_hooks_parts_come_straight_out() {
    let p = Profile { seg: 32, ..Profile::default() };
    let plan = plan(&wall_project(false, vec![tray(0, 0, [2, 1])])).unwrap();
    assert!(plan.parts[0].name.starts_with("wandplatte_1A_3x3_magnete"), "{}", plan.parts[0].name);
    assert!(!plan.parts[0].name.contains("_haken"), "{}", plan.parts[0].name);
    assert!(plan.parts[1].name.ends_with("_ohne_haken.stl"), "{}", plan.parts[1].name);
    let (plate, part) = seated(&plan, &p);
    assert!(overlap(&plate, &part, 0.0, 0.0) < 0.01);
    assert!(overlap(&plate, &part, 0.0, 1.0) < 0.01);
}

#[test]
fn the_plate_has_a_hook_in_every_cell() {
    let p = Profile { seg: 32, ..Profile::default() };
    let with = plan(&wall_project(true, vec![])).unwrap();
    let without = plan(&wall_project(false, vec![])).unwrap();
    assert_eq!(with.parts[0].name, "wandplatte_1A_3x3_magnete_haken.stl");
    let volume = |plan: &Plan| all_parts(plan, &p)[0].mesh().unwrap().volume();
    // The stem reaches 0.01 mm into the floor, which the plate has already.
    let hook = wall::hook().volume() - bingrove_core::spec::HOOK_W * bingrove_core::spec::HOOK_STEM * 0.01;
    let added = volume(&with) - volume(&without);
    assert!((added - 9.0 * hook).abs() < 0.5, "hooks add {added:.1} mm³, nine are {:.1}", 9.0 * hook);
    // The hook stays below the plate's top, inside the pocket.
    let hook_top = wall::hook().bounding_box().max.z;
    assert!(hook_top < bingrove_core::spec::BP_H - 1.5, "{hook_top}");
    // Without parts every cell stays free.
    assert_eq!(with.placements.len(), 0);
}

#[test]
fn trays_are_watertight_and_sized_as_planned() {
    let p = Profile { seg: 32, ..Profile::default() };
    let mut open = tray(0, 0, [2, 1]);
    open.bin.dividers = [3, 1];
    open.bin.open_front = true;
    open.bin.label_text = Some("Schrauben".into());
    open.bin.lip = true;
    open.bin.label_tab = true;
    let plan = plan(&wall_project(true, vec![open, tray(0, 1, [1, 2])])).unwrap();
    let parts = all_parts(&plan, &p);
    for (i, entry) in plan.parts.iter().enumerate().skip(1) {
        let PartSpec::WallBin { spec, hooks } = &entry.part else { panic!("{entry:?}") };
        assert!(hooks);
        assert!(!spec.lip && !spec.label_tab, "a wall bin has no lip or shelf");
        let mesh = parts[i].mesh().unwrap();
        assert!(mesh.is_closed(), "{}", entry.name);
        let (lo, hi) = mesh.bbox();
        let size = wall_size(spec.cells, spec.units);
        assert_eq!(entry.size, size);
        for k in 0..3 {
            assert!(lo[k].abs() < 1e-3, "{} min {lo:?}", entry.name);
            assert!((hi[k] as f64 - size[k]).abs() < 1e-3, "{} max {hi:?} size {size:?}", entry.name);
        }
        // Open at the top: much less than the block it is cut from.
        assert!(mesh.volume() < size[0] * size[1] * size[2] * 0.5, "{}", entry.name);
    }
    assert_eq!(plan.parts[1].name, "wandbin_2x1_6u_3x1fach_offen_Schrauben_magnete_ecken.stl");
    assert_eq!(plan.parts[2].name, "wandbin_1x2_6u_magnete_ecken.stl");
    assert_eq!(plan.parts[1].size[1], 6.0 * UNIT);
}

#[test]
fn the_wall_assembly_stands_up() {
    let p = Profile { seg: 24, ..Profile::default() };
    let plan = plan(&wall_project(true, vec![tray(0, 0, [2, 1]), tray(2, 2, [1, 1])])).unwrap();
    let parts = all_parts(&plan, &p);
    let meshes: Vec<_> = parts.iter().map(|part| part.mesh().unwrap()).collect();
    let (lo, hi) = assemble(&plan, &parts, &meshes).bbox();
    // Up the wall (z) as high as the plate, out of it (−y) as far as plate and trays reach.
    assert!(lo[2].abs() < 1e-3 && (hi[2] - 126.0).abs() < 1e-3, "{lo:?} {hi:?}");
    assert!(hi[1].abs() < 1e-3, "{hi:?}");
    let reach = plan.grid.floor + 6.0 * UNIT;
    assert!((-lo[1] as f64 - reach).abs() < 1e-3, "{lo:?} vs {reach}");
    // The tray in the top cell ends HOOK_LIFT + BIN_GAP under the plate's top.
    let top = 126.0 - BIN_GAP - HOOK_LIFT;
    let tray_top = meshes[2].bbox().1[2] as f64 + plan.position(&plan.placements[1])[1];
    assert!((tray_top - top).abs() < 1e-3, "{tray_top} vs {top}");
}

#[test]
fn old_projects_are_drawers_with_hooks_on() {
    let o: PlateOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(o.kind, Kind::Drawer);
    assert!(o.hooks);
    // A drawer never gets hooks or wall parts, whatever the option says.
    let mut project = wall_project(true, vec![tray(0, 0, [1, 1])]);
    project.options.kind = Kind::Drawer;
    project.options.cover = bingrove_core::model::Cover::Partial;
    let plan = plan(&project).unwrap();
    assert!(!plan.plate.hooks && !plan.plate.wall);
    assert!(plan.parts.iter().all(|e| !e.part.is_wall()));
}
