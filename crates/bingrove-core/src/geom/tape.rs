//! Tape holders: a flat bin with one ring post per stack of rolls; the post reaches into the
//! cardboard core of the top roll.

use super::Part;
use super::bin::feet;
use super::kernel::{cylinder, rrect_prism, translate, union};
use crate::model::TapeSpec;
use crate::profile::Profile;
use crate::spec::*;

pub fn tape(t: &TapeSpec, p: &Profile) -> Part {
    let seg = p.seg * 2;
    let cells = t.cells();
    let [w, d] = t.outer();

    let floor = translate(&rrect_prism(w, d, BIN_R, FLOOR, p.seg), [0.0, 0.0, FOOT_H]);

    let r = t.core / 2.0 - TAPE_POST_GAP;
    let ri = r - TAPE_POST_WALL;
    let h = TAPE_POST_H + (t.height as f64 - 1.0) * TAPE_W;
    let shaft = cylinder(h - TAPE_CHAMFER + 0.01, r, r, seg)
        + translate(&cylinder(TAPE_CHAMFER, r, r - TAPE_CHAMFER, seg), [0.0, 0.0, h - TAPE_CHAMFER]);
    let bore = translate(&cylinder(h + 2.0, ri, ri, seg), [0.0, 0.0, -1.0]);
    // 45° reinforcement inside the foot of the post.
    let fillet = cylinder(TAPE_FILLET, r, r, seg) - cylinder(TAPE_FILLET + 0.01, ri - TAPE_FILLET, ri, seg);
    let post = (shaft - bore) + fillet;

    let [nx, ny] = t.stacks;
    let mut parts = feet(cells, p.seg);
    parts.push(floor);
    for i in 0..nx {
        for j in 0..ny {
            let x = -w / 2.0 + (i as f64 + 0.5) * w / nx as f64;
            let y = -d / 2.0 + (j as f64 + 0.5) * d / ny as f64;
            parts.push(translate(&post, [x, y, FLOOR_Z - 0.01]));
        }
    }
    Part::new(crate::names::tape(t), translate(&union(parts), [w / 2.0, d / 2.0, 0.0]))
}
