//! The space a project is planned for, shown round the assembly: the assembly lies inside it
//! when the plan fits, and sticks out where a grid was made larger than its surface.

use bingrove_core::geom::{all_parts, assemble};
use bingrove_core::model::{BinSpec, DrawerProject, DrawerSpec, Kind, MagnetMode, Placement, PlateFloor, PlateOptions};
use bingrove_core::plan::{plan, space};
use bingrove_core::presets::Preset;
use bingrove_core::profile::Profile;
use bingrove_core::warn::Warning;

/// The assembly's bounding box and the project's space.
fn boxes(project: &DrawerProject) -> ([[f64; 3]; 2], [[f64; 3]; 2]) {
    let p = Profile { seg: 16, ..Profile::default() };
    let plan = plan(project).unwrap();
    let parts = all_parts(&plan, &p);
    let meshes: Vec<_> = parts.iter().map(|part| part.mesh().unwrap()).collect();
    let (lo, hi) = assemble(&plan, &parts, &meshes).bbox();
    ([lo.map(f64::from), hi.map(f64::from)], space(project, &plan).unwrap())
}

fn inside(model: &[[f64; 3]; 2], room: &[[f64; 3]; 2]) -> bool {
    (0..3).all(|k| model[0][k] >= room[0][k] - 0.01 && model[1][k] <= room[1][k] + 0.01)
}

#[test]
fn a_drawer_surrounds_its_insert_with_the_clearance_all_round() {
    let project = DrawerProject {
        drawer: DrawerSpec { name: "Kleinkram".into(), width: 250.0, depth: 305.0, height: 100.0 },
        placements: Preset::get("250x305").unwrap().placements,
        ..DrawerProject::default()
    };
    let (model, room) = boxes(&project);
    assert!(inside(&model, &room), "{model:?} in {room:?}");
    let size: Vec<f64> = (0..3).map(|k| room[1][k] - room[0][k]).collect();
    assert_eq!(size, [250.0, 305.0, 100.0]);
    // The plate is the drawer less its clearance, half of it on either side; where edge bins
    // reach to the drawer's wall they keep their own gap on top of that.
    let c = project.options.clearance / 2.0;
    for k in 0..2 {
        for gap in [model[0][k] - room[0][k], room[1][k] - model[1][k]] {
            assert!(gap > c - 0.01 && gap < c + 0.6, "axis {k}: {gap} mm between {model:?} and {room:?}");
        }
    }
    assert!((model[0][1] - room[0][1] - c).abs() < 0.01, "{model:?} in {room:?}");
    assert_eq!(room[0][2], 0.0);
}

#[test]
fn a_grid_larger_than_its_surface_sticks_out_and_is_warned_about() {
    let surface = |cells| DrawerProject {
        drawer: DrawerSpec { name: "Regal".into(), width: 240.0, depth: 200.0, height: 0.0 },
        options: PlateOptions { kind: Kind::Surface, floor: PlateFloor::Solid, cells, ..PlateOptions::default() },
        ..DrawerProject::default()
    };
    // As many cells as the surface holds: 5 x 4, in its middle, on a flat frame.
    let fits = surface(None);
    let plan_ = plan(&fits).unwrap();
    assert_eq!(plan_.grid.cells, [5, 4]);
    assert!(!plan_.warnings.iter().any(|w| matches!(w, Warning::GridLargerThanSpace { .. })), "{:?}", plan_.warnings);
    let (model, room) = boxes(&fits);
    assert_eq!((room[0][2], room[1][2]), (0.0, 0.0));
    assert!((0..2).all(|k| model[0][k] >= room[0][k] && model[1][k] <= room[1][k]), "{model:?} in {room:?}");
    assert!((model[0][0] - room[0][0] - 15.0).abs() < 0.01 && (room[1][0] - model[1][0] - 15.0).abs() < 0.01);
    // One cell more each way: 252 x 210 mm on 240 x 200, six and five millimetres over a side.
    let large = surface(Some([6, 5]));
    let plan_ = plan(&large).unwrap();
    let over: Vec<_> = plan_
        .warnings
        .iter()
        .filter_map(|w| match w {
            Warning::GridLargerThanSpace { axis, grid, space, wall: false } => Some((*axis, *grid, *space)),
            _ => None,
        })
        .collect();
    assert_eq!(over, [(0, 252.0, 240.0), (1, 210.0, 200.0)]);
    assert!(plan_.warnings[0].german().contains("252 mm breit") && plan_.warnings[0].german().contains("12 mm über"));
    let (model, room) = boxes(&large);
    assert!((room[0][0] - model[0][0] - 6.0).abs() < 0.01 && (model[1][1] - room[1][1] - 5.0).abs() < 0.01);
    // Without a size there is nothing to show or to compare with.
    let mut unsized_ = surface(Some([3, 2]));
    unsized_.drawer.width = 0.0;
    let plan_ = plan(&unsized_).unwrap();
    assert!(space(&unsized_, &plan_).is_none());
    assert!(!plan_.warnings.iter().any(|w| matches!(w, Warning::GridLargerThanSpace { .. })), "{:?}", plan_.warnings);
}

#[test]
fn a_wall_stands_behind_its_parts_with_the_room_in_front() {
    let bin = BinSpec { cells: [1, 1], units: 6, magnets: MagnetMode::Corners, ..BinSpec::default() };
    let project = DrawerProject {
        drawer: DrawerSpec { name: "Wand".into(), width: 150.0, depth: 126.0, height: 60.0 },
        options: PlateOptions { kind: Kind::Wall, floor: PlateFloor::Magnets, ..PlateOptions::default() },
        placements: vec![Placement::bin(1, 1, bin)],
        ..DrawerProject::default()
    };
    let (model, room) = boxes(&project);
    // 150 mm across hold three cells with 12 mm either side; the wall's face is y = 0 and the
    // room reaches 60 mm out of it towards the viewer, the wall's height is z.
    assert_eq!(room, [[-12.0, -60.0, 0.0], [138.0, 0.0, 126.0]]);
    assert!(inside(&model, &room), "{model:?} in {room:?}");
    // A part reaching further out than the room shows through its front.
    let mut deep = project.clone();
    deep.drawer.height = 20.0;
    let (model, room) = boxes(&deep);
    assert!(model[0][1] < room[0][1] - 5.0, "{model:?} in {room:?}");
}
