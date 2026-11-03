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
    let plate = baseplate::plate(&plan.grid, &plan.claims, plan.plate, &profile);
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
    let pieces = baseplate::pieces_of(&plan.grid, &plan.claims, &plan.pieces, plan.plate, &profile);
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

#[test]
fn full_floor_and_magnets_below() {
    use bingrove_core::model::{Kind, PlateStyle};
    let profile = Profile::default();
    let project = DrawerProject {
        drawer: DrawerSpec { name: "Platte".into(), width: 126.0, depth: 84.0, height: 50.0 },
        options: PlateOptions { kind: Kind::Surface, floor: PlateFloor::Solid, ..PlateOptions::default() },
        ..DrawerProject::default()
    };
    let volume = |options: PlateOptions| {
        let plan = plan_for(&project, options);
        let parts = all_parts(&plan, &profile);
        assert!(parts.iter().all(|p| p.mesh().unwrap().is_closed()));
        (plan.parts[0].name.clone(), parts.iter().map(|p| p.solid.volume()).sum::<f64>())
    };
    let (skeleton_name, skeleton) = volume(project.options.clone());
    let (full_name, full) = volume(PlateOptions { style: PlateStyle::Full, ..project.options.clone() });
    assert!(
        skeleton_name.ends_with("_boden.stl") && full_name.ends_with("_boden_voll.stl"),
        "{skeleton_name} {full_name}"
    );
    assert!(full > skeleton + 1000.0, "a full floor carries more: {full} vs {skeleton}");
    let (below_name, below) = volume(PlateOptions { magnets_below: true, ..project.options.clone() });
    assert!(below_name.ends_with("_magnete_unten.stl"), "{below_name}");
    let pocket = std::f64::consts::PI * (profile.magnet.d / 2.0).powi(2) * profile.magnet.h;
    assert!(skeleton - below > 6.0 * 4.0 * pocket * 0.8, "pockets below missing: {skeleton} -> {below}");
    // An open frame ignores all of it.
    let open = PlateOptions {
        floor: PlateFloor::Open,
        magnets_below: true,
        style: PlateStyle::Full,
        ..project.options.clone()
    };
    assert!(volume(open).0.ends_with("_1A_3x2.stl"));
}

fn plan_for(project: &DrawerProject, options: PlateOptions) -> Plan {
    plan(&DrawerProject { options, ..project.clone() }).unwrap()
}

#[test]
fn sides_open_front_fingers_and_window() {
    use bingrove_core::model::{Side, Window};
    let profile = Profile::default();
    let base = BinSpec { cells: [2, 2], units: 6, dividers: [2, 2], ..BinSpec::default() };
    let volume = |b: &BinSpec| {
        let part = bin::bin(b, &profile);
        let mesh = part.mesh().unwrap();
        assert!(mesh.is_closed(), "{} is not watertight", part.name);
        (part.name, mesh.volume(), mesh.bbox())
    };
    let (name, v0, bb0) = volume(&base);
    assert_eq!(name, "bin_2x2_6u_2x2fach.stl");
    // A scoop on the left adds about what one on the front adds; a label on the right its shelf.
    let (name, vf, _) = volume(&BinSpec { scoop: 60, ..base.clone() });
    assert_eq!(name, "bin_2x2_6u_2x2fach_mulde60.stl");
    let (name, vl, _) = volume(&BinSpec { scoop: 60, scoop_sides: vec![Side::Left], ..base.clone() });
    assert_eq!(name, "bin_2x2_6u_2x2fach_mulde60L.stl");
    assert!((vl - vf).abs() < (vf - v0) * 0.1, "left {vl} vs front {vf} over plain {v0}");
    let (name, vr, _) = volume(&BinSpec { label_tab: true, label_side: Side::Right, ..base.clone() });
    assert_eq!(name, "bin_2x2_6u_2x2fach_labelR.stl");
    let (_, vb, _) = volume(&BinSpec { label_tab: true, ..base.clone() });
    assert!((vr - vb).abs() < (vb - v0) * 0.1, "right {vr} vs back {vb}");
    // An open front takes the front wall down; the bin stays as wide and deep.
    let (name, vo, bbo) = volume(&BinSpec { open_front: true, ..base.clone() });
    assert_eq!(name, "bin_2x2_6u_2x2fach_offen.stl");
    assert!(vo < v0 - 500.0, "open front: {vo} vs {v0}");
    assert_eq!((bbo.0, bbo.1[0], bbo.1[1]), (bb0.0, bb0.1[0], bb0.1[1]));
    // Finger notches and a window take material too.
    let (name, vn, _) = volume(&BinSpec { finger_cutouts: true, ..base.clone() });
    assert_eq!(name, "bin_2x2_6u_2x2fach_finger.stl");
    assert!(vn < v0 - 50.0, "fingers: {vn} vs {v0}");
    let (name, vw, _) = volume(&BinSpec { window: Some(Window::default()), ..base.clone() });
    assert_eq!(name, "bin_2x2_6u_2x2fach_fenster20.stl");
    assert!(vw < v0 - 200.0 && vw > vo, "window: {vw} between open {vo} and plain {v0}");
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

#[test]
fn a_lid_sits_on_its_bin_and_carries_another() {
    use bingrove_core::geom::lid;
    use bingrove_core::model::{LidSpec, Placement};
    use bingrove_core::spec::{LID_PLUG, LIP_H};
    let profile = Profile::default();
    let spec = BinSpec { cells: [2, 1], units: 6, ..BinSpec::default() };
    let bin_part = bin::bin(&spec, &profile);
    let lid_part = lid::lid(&LidSpec { cells: [2, 1], clicks: false, ..LidSpec::default() }, &profile);
    let clicked = lid::lid(&LidSpec { cells: [2, 1], ..LidSpec::default() }, &profile);
    let (bm, lm) = (bin_part.mesh().unwrap(), lid_part.mesh().unwrap());
    assert!(lm.is_closed(), "the lid is not watertight");
    assert!(clicked.mesh().unwrap().is_closed());
    assert_eq!(clicked.name, "deckel_2x1_klick.stl");
    assert_eq!(lid_part.name, "deckel_2x1.stl");
    let bin_top = bm.bbox().1[2] as f64;
    // The lid's plug goes LID_PLUG deep into the bin's lip without touching it; only the click
    // bumps press into the lip, and only a little.
    let on = lid_part.solid.translate(Vec3::new(0.0, 0.0, bin_top - LID_PLUG));
    let hit = (&on ^ &bin_part.solid).volume();
    assert!(hit < 1e-3, "the lid cuts {hit:.3} mm³ into its bin");
    let bumps = (&clicked.solid.translate(Vec3::new(0.0, 0.0, bin_top - LID_PLUG)) ^ &bin_part.solid).volume();
    assert!(bumps > 0.5 && bumps < 40.0, "the click bumps press {bumps:.2} mm³ into the lip");
    // Another bin stands on the lid: its foot sinks at least 3.5 mm into the lid's lip.
    let lid_top = bin_top - LID_PLUG + lm.bbox().1[2] as f64;
    let sunk = (0..=12)
        .map(|k| 3.5 + k as f64 * 0.1)
        .rev()
        .find(|&s| (&bin_part.solid.translate(Vec3::new(0.0, 0.0, lid_top - s)) ^ &on).volume() < 1e-3)
        .unwrap_or(0.0);
    assert!(sunk >= 3.5, "a bin on the lid sinks only {sunk} mm into the lip (lip is {LIP_H})");
    // A plan prints one lid kind per footprint, as often as asked.
    let mut project = DrawerProject {
        drawer: DrawerSpec { name: "Deckel".into(), width: 150.0, depth: 108.0, height: 60.0 },
        ..DrawerProject::default()
    };
    project.placements = vec![
        Placement { lid: true, ..Placement::bin(0, 0, BinSpec { cells: [2, 1], units: 6, ..BinSpec::default() }) },
        Placement { lid: true, ..Placement::bin(0, 1, BinSpec { cells: [2, 1], units: 6, ..BinSpec::default() }) },
        Placement::bin(2, 0, BinSpec { cells: [1, 2], units: 6, ..BinSpec::default() }),
    ];
    let plan = plan(&project).unwrap();
    let lids: Vec<_> = plan.parts.iter().filter(|b| b.name.starts_with("deckel_")).collect();
    assert_eq!(lids.len(), 1, "{:?}", plan.parts.iter().map(|b| &b.name).collect::<Vec<_>>());
    assert_eq!(lids[0].quantity, 2);
    assert!(plan.parts.iter().all(|b| b.size[2] > 0.0));
}
