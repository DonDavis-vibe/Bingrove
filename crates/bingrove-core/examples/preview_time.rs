//! How long the panel's preview takes natively (wasm is about 1.5–3× slower).

use std::time::Instant;

use bingrove_core::geom;
use bingrove_core::model::{DrawerProject, DrawerSpec, PlateFloor};
use bingrove_core::plan::plan;
use bingrove_core::profile::Profile;

fn main() {
    for (w, d) in [(250.0, 305.0), (435.0, 330.0)] {
        for floor in [PlateFloor::Open, PlateFloor::Magnets] {
            let mut project = DrawerProject {
                drawer: DrawerSpec { name: "t".into(), width: w, depth: d, height: 100.0 },
                ..DrawerProject::default()
            };
            project.options.floor = floor;
            let plan = plan(&project).expect("plan");
            for seg in [24u32, 16] {
                let p = Profile { seg, ..Profile::default() };
                let t = Instant::now();
                let mesh = geom::assembly_mesh(&plan, &p).expect("mesh");
                println!(
                    "{w}x{d} {floor:?} seg {seg}: {:.2} s, {} parts, {} triangles",
                    t.elapsed().as_secs_f64(),
                    plan.parts.len(),
                    mesh.indices.len() / 3
                );
            }
        }
    }
}
