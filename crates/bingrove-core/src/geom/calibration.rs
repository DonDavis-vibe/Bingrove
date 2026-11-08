//! Test pieces to find the right numbers for a printer: a ladder of magnet holes and a fit
//! test of bins with different air to the baseplate.

use super::Part;
use super::kernel::{Solid, Vec3, cuboid, cylinder, rrect_prism, subtract, swept, translate, union};
use super::text;
use crate::model::Drive;
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
        let label = format!("{dia:.1}");
        let body = text::text_solid(&label, 4.0, text::stroke_for(4.0), 0.6 + 1.0);
        cuts.push(translate(&body, [x - text::text_width(&label, 4.0) / 2.0, 2.5, h - 0.6]));
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
    out.extend(connector_pair(p));
    out.push(peg_bar(p));
    out.push(wall_cell(p));
    out.extend(HOOK_GAPS.iter().map(|&g| super::wall::hook_test(g, p)));
    out.extend(LATCH_DETENTS.iter().map(|&d| super::wall::latch_test(d, p)));
    out
}

/// Air around the hook tried by the wall test blocks.
pub const HOOK_GAPS: [f64; 3] = [0.2, 0.3, 0.4];

/// How far the latch rib of the wall test blocks pushes the blade aside.
pub const LATCH_DETENTS: [f64; 3] = [0.3, 0.5, 0.7];

/// One cell of a wall plate, floor with magnets, the hook and the latch blade, to hang the test
/// blocks on.
pub fn wall_cell(p: &Profile) -> Part {
    use crate::model::{Kind, PlateFloor, PlateOptions};
    let o =
        PlateOptions { kind: Kind::Wall, floor: PlateFloor::Magnets, cells: Some([1, 1]), ..PlateOptions::default() };
    let (grid, _) = crate::grid::Grid::new([GRID, GRID], &o, &p.magnet).expect("one cell fits");
    let claims = crate::plan::EdgeClaims { sides: Default::default(), corners: [false; 4] };
    Part::new("kalibrierung_wand_zelle.stl".into(), super::baseplate::plate(&grid, &claims, o.build(), p))
}

/// Undersizes tried by the peg bar, mm below the drive square.
pub const PEG_UNDERSIZES: [f64; 3] = [0.2, 0.3, 0.4];

/// A bar with three pegs per drive, each a little narrower than the drive square, the drive
/// and the undersize engraved in front of it: which one a socket slides on and still holds.
pub fn peg_bar(p: &Profile) -> Part {
    let pitch = 20.0;
    let n = Drive::ALL.len() * PEG_UNDERSIZES.len();
    let (w, d, h) = (n as f64 * pitch + 4.0, 32.0, 3.0);
    let mut parts = vec![translate(&rrect_prism(w, d, 2.0, h, p.seg), [w / 2.0, d / 2.0, 0.0])];
    let mut labels = Vec::new();
    let mut i = 0;
    for drive in Drive::ALL {
        for under in PEG_UNDERSIZES {
            let x = 2.0 + (i as f64 + 0.5) * pitch;
            let side = drive.square() - under;
            let c = (side / 2.0 - 0.8).max(1.0);
            let corners =
                |r: f64, z: f64| [[-r, -r], [r, -r], [r, r], [-r, r]].map(|[dx, dy]| Vec3::new(x + dx, 20.0 + dy, z));
            let mut pts: Vec<Vec3> = corners(side / 2.0, h - 1.0).to_vec();
            pts.extend(corners(side / 2.0, h + PEG_H - 0.8));
            pts.extend(corners(c, h + PEG_H));
            parts.push(Solid::hull(&pts));
            let label = format!("{} -{}", drive.label().trim_end_matches('"'), crate::names::mm(under));
            let height = text::fit_height(&label, 3.0, pitch - 2.0);
            let body = text::text_solid(&label, height, text::stroke_for(height), 0.6 + 1.0);
            labels.push(translate(&body, [x - text::text_width(&label, height) / 2.0, 4.0, h - 0.6]));
            i += 1;
        }
    }
    Part::new("kalibrierung_stecker.stl".into(), subtract(union(parts), labels))
}

/// Two slabs of plate floor to try the dovetail connector: "a" carries the tab on its right
/// edge, "b" the pocket on its left, both lettered.
pub fn connector_pair(p: &Profile) -> Vec<Part> {
    let fl = p.magnet.h + p.magnet.floor;
    let (w, d) = (24.0, 24.0);
    let tail = |grow: f64| -> Solid {
        let (n, hd, dp) = (CONNECTOR_NECK / 2.0 + grow, CONNECTOR_HEAD / 2.0 + grow, CONNECTOR_DEPTH + grow);
        let pts = vec![[-1.0, d / 2.0 - n], [dp, d / 2.0 - hd], [dp, d / 2.0 + hd], [-1.0, d / 2.0 + n]];
        super::baseplate::dovetail_prism(&pts, fl, grow > 0.0)
    };
    let slab = || translate(&rrect_prism(w, d, 2.0, fl, p.seg), [w / 2.0, d / 2.0, 0.0]);
    let letter = |s: Solid, t: &str| text::engrave(s, t, [w / 2.0, d / 2.0, fl], 6.0, 0.6);
    let a = letter(slab() + translate(&tail(0.0), [w, 0.0, 0.0]), "a");
    let b = letter(slab() - tail(CONNECTOR_GAP), "b");
    vec![Part::new("kalibrierung_verbinder_a.stl".into(), a), Part::new("kalibrierung_verbinder_b.stl".into(), b)]
}

/// The solids only, for tests.
pub fn solids(p: &Profile) -> Vec<Solid> {
    parts(p).into_iter().map(|part| part.solid).collect()
}
