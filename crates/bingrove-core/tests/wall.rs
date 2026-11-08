//! Wall plates and the parts that hang on them: the hook lets a part in when it is lifted, keeps
//! it from being pulled straight out when it hangs, and the assembly stands up.

use bingrove_core::geom::kernel::{Solid, Vec3};
use bingrove_core::geom::{all_parts, assemble, wall};
use bingrove_core::model::{
    BinSpec, DrawerProject, DrawerSpec, HolderSpec, Kind, MagnetMode, Placement, PlateFloor, PlateOptions,
    WallHolesSpec, WallVSlotSpec,
};
use bingrove_core::plan::{PartSpec, Plan, plan};
use bingrove_core::profile::Profile;
use bingrove_core::spec::{BIN_GAP, HOOK_LIFT, UNIT, wall_size};
use bingrove_core::warn::Warning;

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
    // Lifted, it goes straight in past the hook: pushed up to its stop (the foot's top meets the
    // pocket) or anywhere a millimetre short of it ...
    for lift in [HOOK_LIFT - 0.8, HOOK_LIFT, HOOK_LIFT + BIN_GAP] {
        for dz in [8.0, 6.0, 4.0, 2.5, 1.0, 0.0] {
            let v = overlap(&plate, &part, lift, dz);
            assert!(v < 0.01, "going in lifted by {lift}, {dz} mm out: {v} mm³");
        }
    }
    // ... past its stop the pocket's top holds it, and not lifted far enough the tooth meets the hook.
    assert!(overlap(&plate, &part, HOOK_LIFT + BIN_GAP + 0.3, 0.0) > 0.01, "no stop above");
    assert!(overlap(&plate, &part, 1.0, 1.5) > 1.0, "the tooth lets it in lifted by 1 mm");
    // ... and slides down onto its seat.
    for k in 0..=12 {
        let dy = HOOK_LIFT - k as f64 * (HOOK_LIFT - settled) / 12.0;
        let v = overlap(&plate, &part, dy, 0.0);
        assert!(v < 0.01, "sliding down at {dy:.2} mm: {v} mm³");
    }
    // Hanging, pulling it straight out runs the feet into the hooks' fingers.
    let pulled = overlap(&plate, &part, 0.0, 1.0);
    assert!(pulled > 5.0, "pulled out 1 mm: only {pulled} mm³ in the way");
    let pulled = overlap(&plate, &part, settled, 1.0);
    assert!(pulled > 5.0, "settled and pulled out 1 mm: only {pulled} mm³ in the way");
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
    assert!(hook_top <= bingrove_core::spec::BP_H - 1.2, "{hook_top}");
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

fn wall_sets() -> Vec<bingrove_core::catalog::HolderSet> {
    bingrove_core::catalog::sets().into_iter().filter(|s| s.holder.is_wall()).collect()
}

#[test]
fn wall_holders_are_watertight_and_sized_as_planned() {
    let p = Profile { seg: 24, ..Profile::default() };
    let mut holders: Vec<(String, HolderSpec)> =
        wall_sets().into_iter().map(|s| (s.id.to_string(), s.holder)).collect();
    holders.extend(
        bingrove_core::catalog::catalog()
            .into_iter()
            .filter(|e| e.holder.is_wall())
            .map(|e| (e.id.to_string(), e.holder)),
    );
    assert!(holders.len() >= 12, "{}", holders.len());
    for (id, h) in &holders {
        let part = wall::holder(h, true, MagnetMode::Corners, &p);
        assert!(part.name.starts_with("wandhalter_"), "{id}: {}", part.name);
        let mesh = part.mesh().unwrap();
        assert!(mesh.is_closed(), "{id}");
        let (lo, hi) = mesh.bbox();
        let size = h.size();
        for k in 0..3 {
            assert!(lo[k].abs() < 1e-3, "{id} min {lo:?}");
            assert!((hi[k] as f64 - size[k]).abs() < 1e-3, "{id} max {hi:?} size {size:?}");
        }
        // The block in front of the plate fits the part.
        let block = match h {
            HolderSpec::WallHoles(s) => s.block(),
            HolderSpec::WallCradles(s) => s.block(),
            HolderSpec::WallVSlots(s) => s.block(),
            _ => unreachable!(),
        };
        assert!(block[0] <= size[0] + 1e-6, "{id}: {block:?} wider than {size:?}");
        assert!(block[1] <= size[1] - 5.95 + 1e-6, "{id}: {block:?} deeper than {size:?}");
        assert!(block[2] <= size[2] + 1e-6, "{id}: {block:?} taller than {size:?}");
    }
}

/// Volume of `solid` inside a box.
fn inside(solid: &Solid, size: [f64; 3], at: [f64; 3]) -> f64 {
    (solid ^ &bingrove_core::geom::kernel::cuboid(size, at)).volume()
}

#[test]
fn holes_go_down_and_through_holes_come_out_below() {
    let p = Profile { seg: 32, ..Profile::default() };
    let spec = WallHolesSpec { d: 10.0, depth: 20.0, count: [3, 1], ..WallHolesSpec::default() }.fit();
    let h = HolderSpec::WallHoles(spec.clone());
    let solid = wall::holder(&h, true, MagnetMode::None, &p).solid;
    let [w, _, _] = h.size();
    let back = h.size()[1] - 5.95;
    let [_, _, top] = spec.block();
    // Over the middle hole, near its mouth: empty; under it in the floor: solid.
    let probe = [2.0, 2.0, 2.0];
    let at = |z: f64| [w / 2.0 - 1.0, back / 2.0 - 1.0, z];
    assert!(inside(&solid, probe, at(top - 3.0)) < 0.01);
    assert!((inside(&solid, [2.0, 2.0, 0.6], at(0.3)) - 2.4).abs() < 0.01);
    // Through: the same hole open at the bottom.
    let through = HolderSpec::WallHoles(WallHolesSpec { through: true, ..spec }.fit());
    let solid = wall::holder(&through, true, MagnetMode::None, &p).solid;
    assert!(inside(&solid, [2.0, 2.0, 0.6], at(0.3)) < 0.01);
}

#[test]
fn cradles_step_up_and_vslots_open_to_the_front() {
    let p = Profile { seg: 32, ..Profile::default() };
    let set = |id: &str| wall_sets().into_iter().find(|s| s.id == id).unwrap().holder;
    let h = set("wall_extensions");
    let HolderSpec::WallCradles(c) = &h else { panic!() };
    assert!(c.stepped);
    let solid = wall::holder(&h, true, MagnetMode::None, &p).solid;
    // The front strip of each column ends at its own top, a unit above the one before.
    let w = h.size()[0];
    let [width, _, _] = c.block();
    let mut x = w / 2.0 - width / 2.0;
    let mut tops = Vec::new();
    for slot in &c.slots {
        let place = c.place(slot);
        let column = &solid ^ &bingrove_core::geom::kernel::cuboid([place - 1.0, 2.0, 200.0], [x + 0.5, 0.0, -1.0]);
        tops.push(column.bounding_box().max.z);
        x += place;
    }
    for k in 1..tops.len() {
        assert!((tops[k] - tops[k - 1] - UNIT).abs() < 1e-3, "{tops:?}");
    }
    // Pliers: the slot is open at the front just under the shelf's top.
    let h = set("wall_pliers");
    let HolderSpec::WallVSlots(v) = &h else { panic!() };
    let [width, _, top] = v.block();
    let w = h.size()[0];
    let x = w / 2.0 - width / 2.0 + 1.6 + (v.slots[0].width + v.clearance) / 2.0;
    let solid = wall::holder(&h, true, MagnetMode::None, &p).solid;
    assert!(inside(&solid, [2.0, 2.0, 2.0], [x - 1.0, 0.0, top - 4.0]) < 0.01);
    let closed = HolderSpec::WallVSlots(WallVSlotSpec { closed_front: true, ..v.clone() }.fit());
    let solid = wall::holder(&closed, true, MagnetMode::None, &p).solid;
    assert!(inside(&solid, [1.0, 1.0, 2.0], [x - 0.5, 0.05, top - 4.0]) > 1.9);
}

#[test]
fn a_wall_holder_hangs_like_a_bin() {
    let p = Profile { seg: 32, ..Profile::default() };
    let set = wall_sets().into_iter().find(|s| s.id == "wall_pliers").unwrap().holder;
    let [cw, ch] = set.cells();
    let mut placement = tray(0, 0, [cw, ch]);
    placement.holder = Some(set);
    let mut project = wall_project(true, vec![placement]);
    project.drawer.width = 42.0 * cw.max(3) as f64;
    let plan = plan(&project).unwrap();
    assert!(matches!(plan.parts[1].part, PartSpec::WallHolder { hooks: true, .. }));
    assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    let (plate, part) = seated(&plan, &p);
    assert!(overlap(&plate, &part, 0.0, 0.0) < 0.01);
    assert!(overlap(&plate, &part, HOOK_LIFT, 0.0) < 0.01);
    assert!(overlap(&plate, &part, HOOK_LIFT, 3.0) < 0.01);
    assert!(overlap(&plate, &part, 0.0, 1.0) > 3.0);
}

#[test]
fn holders_in_the_wrong_place_are_warned_about() {
    let wall_holder = wall_sets()[0].holder.clone();
    let mut on_drawer = tray(0, 0, wall_holder.cells());
    on_drawer.holder = Some(wall_holder);
    let mut project = wall_project(true, vec![on_drawer]);
    project.options.kind = Kind::Surface;
    let plan_ = plan(&project).unwrap();
    assert!(plan_.warnings.iter().any(|w| matches!(w, Warning::WallHolderOffWall { .. })), "{:?}", plan_.warnings);
    let drawer_holder = HolderSpec::Holes(bingrove_core::model::HolesSpec::default());
    let mut on_wall = tray(0, 0, drawer_holder.cells());
    on_wall.holder = Some(drawer_holder);
    let plan_ = plan(&wall_project(true, vec![on_wall])).unwrap();
    assert!(plan_.warnings.iter().any(|w| matches!(w, Warning::HolderNotForWall { .. })), "{:?}", plan_.warnings);
}

#[test]
fn the_solver_hangs_sets_and_trays() {
    use bingrove_core::model::{ItemKind, ItemRequest};
    let mut project = wall_project(true, vec![]);
    project.drawer.width = 8.0 * 42.0;
    project.drawer.depth = 6.0 * 42.0;
    project.items = vec![
        ItemRequest {
            name: "Verlängerungen".into(),
            quantity: 1,
            what: ItemKind::Holder { holder: None, set: Some("wall_extensions".into()), count: None },
        },
        ItemRequest {
            name: "Schraubendreher".into(),
            quantity: 1,
            what: ItemKind::Holder { holder: None, set: Some("wall_screwdrivers".into()), count: Some(10) },
        },
        ItemRequest {
            name: "Ablage".into(),
            quantity: 2,
            what: ItemKind::Bin { cells: [2, 1], units: Some(6), dividers: [1, 1] },
        },
    ];
    let plan = plan(&project).unwrap();
    assert_eq!(plan.placements.len(), 4, "{:?}", plan.warnings);
    let names: Vec<_> = plan.parts.iter().map(|e| e.name.as_str()).collect();
    assert!(names.iter().any(|n| n.starts_with("wandhalter_wiegen_Verlaengerungen_gestuft")), "{names:?}");
    assert!(
        names.iter().any(|n| n.starts_with("wandhalter_loecher_Schraubendreher") && n.contains("_durch")),
        "{names:?}"
    );
    assert!(names.contains(&"wandbin_2x1_6u.stl"), "{names:?}");
    // Ten screwdrivers in rows of at most eight.
    let screwdrivers = plan.placements.iter().find_map(|p| match &p.placement.holder {
        Some(HolderSpec::WallHoles(w)) => Some(w.count),
        _ => None,
    });
    assert_eq!(screwdrivers, Some([5, 2]));
}

#[test]
fn vslot_holders_read_both_spellings() {
    for (json, wall) in [
        (r#"{"kind": "vslots"}"#, false),
        (r#"{"kind": "v_slots"}"#, false),
        (r#"{"kind": "wall_vslots"}"#, true),
        (r#"{"kind": "wall_v_slots"}"#, true),
    ] {
        let h: HolderSpec = serde_json::from_str(json).unwrap();
        assert_eq!(h.is_wall(), wall, "{json}");
    }
    let h = HolderSpec::VSlots(Default::default());
    assert!(serde_json::to_string(&h).unwrap().contains(r#""kind":"vslots""#));
}

#[test]
fn skadis_hooks_slide_in_from_behind_and_carry_the_plate() {
    use bingrove_core::model::Mount;
    use bingrove_core::spec::skadis_hook_size;
    let p = Profile { seg: 24, ..Profile::default() };
    let mut project = wall_project(true, vec![]);
    project.drawer.width = 4.0 * 42.0;
    project.drawer.depth = 2.0 * 42.0;
    project.options.mount = Mount { skadis: true, ..Mount::default() };
    let plan = plan(&project).unwrap();
    assert_eq!(plan.parts[0].name, "wandplatte_1A_4x2_magnete_haken_skadis.stl");
    let hooks = plan.parts.iter().find(|e| matches!(e.part, PartSpec::SkadisHook)).unwrap();
    assert_eq!(hooks.name, "skadis_haken.stl");
    assert_eq!(hooks.quantity, 2, "168 mm takes a pair of hooks");
    let parts = all_parts(&plan, &p);
    let hook = parts.iter().find(|part| part.name == "skadis_haken.stl").unwrap();
    let mesh = hook.mesh().unwrap();
    assert!(mesh.is_closed());
    let (lo, hi) = mesh.bbox();
    let size = skadis_hook_size();
    for k in 0..3 {
        assert!(lo[k].abs() < 1e-3 && (hi[k] as f64 - size[k]).abs() < 1e-3, "{lo:?} {hi:?} vs {size:?}");
    }
    let plate = &parts[0];
    assert!(plate.mesh().unwrap().is_closed());
    let plate = plate.solid.translate(Vec3::new(plate.origin[0], plate.origin[1], 0.0));
    let top = 2.0 * 42.0;
    for dx in bingrove_core::spec::skadis_offsets(168.0) {
        let hook = wall::skadis_hook_at(84.0 + dx, top, &p);
        let at = |v: [f64; 3]| (&plate ^ &hook.translate(Vec3::new(v[0], v[1], v[2]))).volume();
        assert!(at([0.0; 3]) < 1e-3, "seated: {}", at([0.0; 3]));
        // In from behind (the plate lies face down, the tab goes into the pocket).
        for dz in [-6.0, -3.0, -1.0] {
            assert!(at([0.0, 0.0, dz]) < 1e-3, "from behind at {dz}: {}", at([0.0, 0.0, dz]));
        }
        // Hanging, the plate pulls down on the hook: the dovetail's head holds it.
        assert!(at([0.0, 1.0, 0.0]) > 1.0, "the plate slips off: {}", at([0.0, 1.0, 0.0]));
    }
}
