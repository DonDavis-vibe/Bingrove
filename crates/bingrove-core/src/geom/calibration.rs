//! Test pieces to find the right numbers for a printer: a ladder of magnet holes and a fit
//! test of bins with different air to the baseplate.

use super::Part;
use super::kernel::{Solid, cuboid, cylinder, digits, rrect_prism, subtract, swept, translate, union};
use crate::profile::Profile;
use crate::spec::*;

/// Diameters of the ladder: the profile's hole ±0.3 mm in steps of 0.1.
pub fn ladder_diameters(p: &Profile) -> Vec<f64> {
    (-3..=3).map(|k| ((p.magnet.d + k as f64 * 0.1) * 10.0).round() / 10.0).collect()
}

/// Air between bin and cell tried by the fit test, per side.
pub const FIT_GAPS: [f64; 3] = [0.15, 0.25, 0.35];

/// A bar with the ladder of holes, each labelled with its diameter.
pub fn magnet_ladder(p: &Profile) -> Part {
    let ds = ladder_diameters(p);
    let pitch = ds.iter().cloned().fold(0.0, f64::max) + 6.0;
    let (w, d, h) = (ds.len() as f64 * pitch + 4.0, pitch + 10.0, p.magnet.h + 2.0);
    let mut cuts = Vec::new();
    for (i, &dia) in ds.iter().enumerate() {
        let x = 2.0 + (i as f64 + 0.5) * pitch;
        cuts.push(translate(
            &cylinder(p.magnet.h + 1.0, dia / 2.0, dia / 2.0, p.seg),
            [x, d - pitch / 2.0 - 2.0, h - p.magnet.h],
        ));
        cuts.push(translate(&digits(&format!("{dia:.1}"), 4.0, 0.6 + 1.0), [x - 4.5, 2.5, h - 0.6]));
    }
    let bar = translate(&rrect_prism(w, d, 2.0, h, p.seg), [w / 2.0, d / 2.0, 0.0]);
    let name = format!("kalibrierung_magnete_{:.1}-{:.1}.stl", ds[0], ds[ds.len() - 1]);
    Part::new(name, subtract(bar, cuts))
}

/// One open baseplate cell to try the fit-test bins in.
pub fn fit_cell(p: &Profile) -> Part {
    let plate = translate(&rrect_prism(GRID, GRID, CORNER_R, BP_H, p.seg), [GRID / 2.0, GRID / 2.0, 0.0]);
    let cell = swept(GRID, GRID, BP_R, &BP_PROFILE, (GRID / 2.0, GRID / 2.0), p.seg);
    Part::new("kalibrierung_passung_feld.stl".into(), plate - cell)
}

/// A low 1x1 bin whose foot has `gap` mm of air to the cell on each side, marked with `notches` notches.
pub fn fit_bin(gap: f64, notches: u32, p: &Profile) -> Part {
    let outer = GRID - 2.0 * gap;
    let units = 2;
    let top = units as f64 * UNIT;
    let foot = swept(outer, outer, BIN_R, &FOOT_PROFILE, (0.0, 0.0), p.seg);
    let body = translate(&rrect_prism(outer, outer, BIN_R, top - FOOT_H, p.seg), [0.0, 0.0, FOOT_H]);
    let cavity =
        translate(&rrect_prism(outer - 2.0 * WALL, outer - 2.0 * WALL, BIN_R - WALL, top, p.seg), [0.0, 0.0, FLOOR_Z]);
    let mut solid = union(vec![foot, body]) - cavity;
    // Notches in the rim: one per 0.1 mm of air above 0.05.
    for k in 0..notches {
        let x = -6.0 + k as f64 * 4.0;
        solid -= cuboid([2.0, 3.0, 2.0], [x - 1.0, outer / 2.0 - 2.0, top - 1.5]);
    }
    let name = format!("kalibrierung_passung_bin_{}.stl", crate::names::mm(gap));
    Part::new(name, translate(&solid, [outer / 2.0, outer / 2.0, 0.0]))
}

/// Everything for one calibration print.
pub fn parts(p: &Profile) -> Vec<Part> {
    let mut out = vec![magnet_ladder(p), fit_cell(p)];
    for (i, gap) in FIT_GAPS.iter().enumerate() {
        out.push(fit_bin(*gap, i as u32 + 1, p));
    }
    out
}

/// The solids only, for tests.
pub fn solids(p: &Profile) -> Vec<Solid> {
    parts(p).into_iter().map(|part| part.solid).collect()
}
