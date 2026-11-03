//! A push-on lid: a plate carrying the stacking lip on top, so a bin stands on the lid as on
//! another bin, and a plug underneath that sits in the bin's own lip, with click bumps on two
//! sides that snap under the lip's narrow part. Minimum corner at the origin, the plug below
//! z = 0 brought up so everything starts at z = 0.

use super::Part;
use super::kernel::{cuboid, rrect_prism, swept, translate, union};
use crate::model::LidSpec;
use crate::profile::Profile;
use crate::spec::*;

/// The lid; `thickness` of plate between the plug and the lip.
pub fn lid(l: &LidSpec, p: &Profile) -> Part {
    let seg = p.seg;
    let (w, d) = (bin_outer(l.cells[0]), bin_outer(l.cells[1]));
    let plug_h = LID_PLUG;
    let plate_top = plug_h + l.thickness;
    let top = plate_top + LIP_H;
    let c = (w / 2.0, d / 2.0);
    // The body with the lip's cavity cut from the top, as in a bin.
    let body = translate(&rrect_prism(w, d, BIN_R, top - plug_h, seg), [c.0, c.1, plug_h]);
    let cavity = swept(
        w,
        d,
        BIN_R,
        &[(top + 1.0, -1.0), (top, 0.0), (top - 1.9, 1.9), (plate_top + 0.7, 1.9), (plate_top, 2.6)],
        c,
        seg,
    );
    // The plug: the lip opening less air, from the plate down; its lowest part flares out a
    // little to click under the lip's narrow band.
    let inset = 1.9 + LID_GAP;
    let plug =
        translate(&rrect_prism(w - 2.0 * inset, d - 2.0 * inset, BIN_R - inset, plug_h + 0.02, seg), [c.0, c.1, 0.0]);
    let mut parts = vec![body - cavity, plug];
    if l.clicks {
        // Two bumps on each long side, in the plug's lowest 0.8 mm, rounded off by the hull.
        let bump = |x: f64, y: f64, along_x: bool| {
            let (sx, sy) = if along_x { (8.0, LID_CLICK + 0.3) } else { (LID_CLICK + 0.3, 8.0) };
            cuboid([sx, sy, 0.8], [x - sx / 2.0, y - sy / 2.0, 0.3])
        };
        let (bx, by) = (inset - LID_CLICK / 2.0, inset - LID_CLICK / 2.0);
        for k in [0.3, 0.7] {
            parts.push(bump(w * k, by, true));
            parts.push(bump(w * k, d - by, true));
            parts.push(bump(bx, d * k, false));
            parts.push(bump(w - bx, d * k, false));
        }
    }
    Part::new(crate::names::lid(l), union(parts))
}
