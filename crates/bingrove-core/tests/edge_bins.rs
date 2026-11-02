//! Edge bins and the plate they sit on: every part watertight, the assembled drawer free of
//! collisions and inside the drawer.

use bingrove_core::geom::kernel::{Solid, Vec3};
use bingrove_core::geom::{all_parts, baseplate, bin};
use bingrove_core::model::{BinSpec, DividerMode, DrawerProject, DrawerSpec, EdgeMode, PlateFloor, PlateOptions};
use bingrove_core::plan::{Plan, plan};
use bingrove_core::presets::Preset;
use bingrove_core::profile::Profile;

fn small(options: PlateOptions) -> DrawerProject {
    DrawerProject {
        drawer: DrawerSpec { name: "Kleinkram".into(), width: 250.0, depth: 305.0, height: 100.0 },
        options,
        placements: Preset::get("250x305").unwrap().placements,
        ..DrawerProject::default()
    }
}

/// Every bin of the plan placed in plate coordinates.
fn placed_bins(plan: &Plan, profile: &Profile) -> Vec<(String, Solid)> {
    let parts = all_parts(plan, profile);
    plan.placements
        .iter()
        .map(|p| {
            let mut s = parts[p.part].solid.clone();
            if p.turned {
                s = s.rotate(0.0, 0.0, 180.0);
            }
            let lo = s.bounding_box().min;
            let to = plan.position(p);
            (parts[p.part].name.clone(), s.translate(Vec3::new(to[0] - lo.x, to[1] - lo.y, to[2] - lo.z)))
        })
        .collect()
}

fn assert_assembles(plan: &Plan) {
    let profile = Profile::default();
    for part in all_parts(plan, &profile) {
        let mesh = part.mesh().unwrap();
        assert!(mesh.is_closed(), "{} is not watertight", part.name);
    }
    let plate = baseplate::plate(&plan.grid, &plan.claims, plan.floor, &profile);
    let bins = placed_bins(plan, &profile);
    let [w, d] = plan.grid.plate;
    for (i, (name, b)) in bins.iter().enumerate() {
        let hit = (&plate ^ b).volume();
        assert!(hit < 1e-3, "{name} cuts {hit:.4} mm³ into the plate");
        let bb = b.bounding_box();
        assert!(bb.min.x >= 0.5 - 1e-6 && bb.min.y >= -1e-6, "{name} sticks out: {:?}", bb.min);
        assert!(bb.max.x <= w - 0.5 + 1e-6 && bb.max.y <= d + 1e-6, "{name} sticks out: {:?}", bb.max);
        for (other, o) in &bins[i + 1..] {
            let hit = (b ^ o).volume();
            assert!(hit < 1e-3, "{name} and {other} overlap by {hit:.4} mm³");
        }
    }
}

#[test]
fn a_mounted_surface_assembles_from_its_pieces() {
    use bingrove_core::model::{BedSpec, Kind, Mount};
    let profile = Profile::default();
    // 6 x 3 cells on a bed that takes 4 x 3: two pieces, joined by connectors, each screwed down.
    let project = DrawerProject {
        drawer: DrawerSpec { name: "Bank".into(), width: 252.0, depth: 126.0, height: 100.0 },
        options: PlateOptions {
            kind: Kind::Surface,
            floor: PlateFloor::Solid,
            mount: Mount { screws: true, connectors: true },
            ..PlateOptions::default()
        },
        bed: BedSpec { x: 180.0, y: 180.0, ..BedSpec::default() },
        ..DrawerProject::default()
    };
    let plan = plan(&project).unwrap();
    assert_eq!(plan.pieces.len(), 2, "{:?}", plan.split);
    assert!(plan.parts[0].name.ends_with("_boden_schrauben_verbinder.stl"), "{}", plan.parts[0].name);
    let pieces = baseplate::pieces_of(&plan.grid, &plan.claims, &plan.pieces, plan.floor, plan.mount, &profile);
    for part in &pieces {
        assert!(part.mesh().unwrap().is_closed(), "{} is not watertight", part.name);
    }
    // Back in plate coordinates the tab of the left piece lies in the pocket of the right one.
    let placed: Vec<Solid> =
        pieces.iter().map(|p| p.solid.translate(Vec3::new(p.origin[0], p.origin[1], 0.0))).collect();
    let hit = (&placed[0] ^ &placed[1]).volume();
    assert!(hit < 1e-3, "the pieces overlap by {hit:.4} mm³");
    let cut = plan.grid.line(0, plan.split[0][0]);
    let left = placed[0].bounding_box();
    assert!(left.max.x > cut + 3.5, "the left piece carries the tab: {} vs cut {cut}", left.max.x);
    assert!((placed[1].bounding_box().min.x - cut).abs() < 1e-6, "the right piece starts at the cut");
    // Screws take material out of a plain floor; connectors only move it around.
    let plain = PlateOptions { mount: Mount::default(), ..project.options.clone() };
    let plan_plain = plan_for(&project, plain);
    let v_plain: f64 = all_parts(&plan_plain, &profile).iter().map(|p| p.solid.volume()).sum();
    let v_mount: f64 = pieces.iter().map(|p| p.solid.volume()).sum();
    assert!(v_mount < v_plain - 100.0, "screw holes missing: {v_mount} vs {v_plain}");
}

fn plan_for(project: &DrawerProject, options: PlateOptions) -> Plan {
    plan(&DrawerProject { options, ..project.clone() }).unwrap()
}

#[test]
fn to_floor_drawer_assembles() {
    let plan = plan(&small(PlateOptions::default())).unwrap();
    assert_assembles(&plan);
    // The plate is one piece wide now and the tray reaches the plate outline.
    let parts = all_parts(&plan, &Profile::default());
    let (lo, hi) = parts[0].mesh().unwrap().bbox();
    assert_eq!(parts[0].name, "baseplate_1A_5x4.stl");
    assert!(((hi[0] - lo[0]) as f64 - 212.0).abs() < 0.01 && ((hi[1] - lo[1]) as f64 - 173.0).abs() < 0.01);
    let tray = parts.iter().find(|p| p.name == "randbin_1x5_6u_L19.25.stl").unwrap();
    let (lo, hi) = tray.mesh().unwrap().bbox();
    assert!(((hi[0] - lo[0]) as f64 - 60.75).abs() < 0.01 && lo[2].abs() < 1e-4);
}

#[test]
fn resting_drawer_assembles() {
    let o = PlateOptions { edge_mode: EdgeMode::Resting, ..PlateOptions::default() };
    let plan = plan(&small(o)).unwrap();
    assert_eq!(plan.split, [vec![3, 2], vec![4, 3]]);
    assert_assembles(&plan);
}

#[test]
fn magnet_floor_drawer_assembles() {
    let o = PlateOptions { floor: PlateFloor::Magnets, ..PlateOptions::default() };
    let plan = plan(&small(o)).unwrap();
    assert!(plan.warnings.iter().all(|w| !matches!(w, bingrove_core::warn::Warning::PlateHeldByBins { .. })));
    assert_assembles(&plan);
}

#[test]
fn corner_bins_fill_the_corner() {
    // Edge zones on all four sides: 3 x 2 cells with 12 mm margins.
    let project = DrawerProject {
        drawer: DrawerSpec { name: "Ecke".into(), width: 151.0, depth: 109.0, height: 60.0 },
        ..DrawerProject::default()
    };
    let plan = plan(&project).unwrap();
    assert!(plan.claims.corners.iter().all(|&c| c));
    assert_assembles(&plan);
}

#[test]
fn label_scoop_and_grid_dividers() {
    let profile = Profile::default();
    let spec = BinSpec {
        cells: [2, 2],
        units: 6,
        label_tab: true,
        scoop: 60,
        divider_mode: DividerMode::Grid,
        ..BinSpec::default()
    };
    let plain = bin::edge_bin(
        &BinSpec { label_tab: false, scoop: 0, divider_mode: DividerMode::Equal, ..spec.clone() },
        [19.25, 0.0, 0.0, 0.0],
        EdgeMode::ToFloor,
        1.0,
        &profile,
    );
    let full = bin::edge_bin(&spec, [19.25, 0.0, 0.0, 0.0], EdgeMode::ToFloor, 1.0, &profile);
    let (pm, fm) = (plain.mesh().unwrap(), full.mesh().unwrap());
    assert!(fm.is_closed());
    assert_eq!(full.name, "randbin_2x2_6u_L19.25_rasterfach_label_mulde60.stl");
    // Two grid walls (one inner grid line each way, one at the margin part), a shelf, a scoop.
    assert!(fm.volume() - pm.volume() > 10_000.0, "{} vs {}", fm.volume(), pm.volume());
    assert_eq!(pm.bbox(), fm.bbox());
}
