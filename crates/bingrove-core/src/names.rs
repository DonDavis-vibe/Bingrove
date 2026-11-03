//! File names of the parts, German and ASCII like the original script, so existing print lists
//! and folders keep their names.

use crate::model::{
    BinSpec, DividerMode, EdgeMode, HolderSpec, LidSpec, MagnetMode, PlateBuild, PlateFloor, PlateStyle, Side, TapeSpec,
};

pub fn baseplate(row: usize, col: usize, cells: [u32; 2], build: PlateBuild) -> String {
    let col = char::from(b'A' + col as u8);
    if build.wall {
        return wall_plate(format!("wandplatte_{}{col}_{}x{}", row + 1, cells[0], cells[1]), build);
    }
    let mut name = format!("baseplate_{}{col}_{}x{}", row + 1, cells[0], cells[1]);
    match build.floor {
        PlateFloor::Open => {}
        PlateFloor::Solid => name += "_boden",
        PlateFloor::Magnets => name += "_magnete",
    }
    if build.floor != PlateFloor::Open {
        if build.style == PlateStyle::Full {
            name += "_voll";
        }
        if build.magnets_below {
            name += "_magnete_unten";
        }
        if build.mount.screws {
            name += "_schrauben";
        }
        if build.mount.connectors {
            name += "_verbinder";
        }
    }
    name + ".stl"
}

/// A wall plate is always full; what tells them apart is the hooks, magnets and mounting.
fn wall_plate(mut name: String, build: PlateBuild) -> String {
    if build.floor == PlateFloor::Magnets {
        name += "_magnete";
    }
    if build.hooks {
        name += "_haken";
    }
    if build.mount.screws {
        name += "_schrauben";
    }
    if build.mount.connectors {
        name += "_verbinder";
    }
    if build.mount.skadis {
        name += "_skadis";
    }
    name + ".stl"
}

/// A bin hanging on a wall plate: compartments, open front, text, magnets; `_ohne_haken` when
/// it has no keyholes (a plate without hooks).
pub fn wall_bin(b: &BinSpec, hooks: bool) -> String {
    let [gx, gy] = b.cells;
    let mut name = format!("wandbin_{gx}x{gy}_{}u", b.units);
    if b.dividers != [1, 1] {
        name += &format!("_{}x{}fach", b.dividers[0], b.dividers[1]);
    }
    if b.open_front {
        name += "_offen";
    }
    if let Some(text) = b.label_text.as_deref().map(slug).filter(|s| !s.is_empty()) {
        name += &format!("_{text}");
    }
    match b.magnets {
        MagnetMode::None => {}
        MagnetMode::All => name += "_magnete",
        MagnetMode::Corners if gx * gy == 1 => name += "_magnete",
        MagnetMode::Corners => name += "_magnete_ecken",
    }
    if !hooks {
        name += "_ohne_haken";
    }
    name + ".stl"
}

pub fn bin(b: &BinSpec) -> String {
    let [gx, gy] = b.cells;
    let name = format!("bin_{gx}x{gy}_{}u", b.units);
    with_options(name, b)
}

/// A bin reaching into the margins: `randbin_` and the extent per side, e.g. `_L19.25`.
pub fn edge_bin(b: &BinSpec, extents: [f64; 4], mode: EdgeMode) -> String {
    if extents.iter().all(|&e| e == 0.0) {
        return bin(b);
    }
    let [gx, gy] = b.cells;
    let mut name = format!("randbin_{gx}x{gy}_{}u", b.units);
    for s in Side::ALL {
        let e = extents[s as usize];
        if e > 0.0 {
            name += &format!("_{}{}", s.letter(), mm(e));
        }
    }
    if mode == EdgeMode::Resting {
        name += "_aufliegend";
    }
    with_options(name, b)
}

/// Millimetres without trailing zeros: 19.25, 19.5, 19.
pub fn mm(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Free text as a file-name piece: ASCII letters and digits, umlauts spelled out, the rest
/// becomes one underscore, at most 24 characters.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let piece: &str = match c {
            'ä' => "ae",
            'ö' => "oe",
            'ü' => "ue",
            'Ä' => "Ae",
            'Ö' => "Oe",
            'Ü' => "Ue",
            'ß' => "ss",
            c if c.is_ascii_alphanumeric() => {
                out.push(c);
                continue;
            }
            _ => "_",
        };
        if piece != "_" || !out.ends_with('_') {
            out.push_str(piece);
        }
    }
    let out = out.trim_matches('_');
    out.chars().take(24).collect::<String>().trim_end_matches('_').to_string()
}

fn with_options(mut name: String, b: &BinSpec) -> String {
    let [gx, gy] = b.cells;
    if b.dividers != [1, 1] {
        name += &format!("_{}x{}fach", b.dividers[0], b.dividers[1]);
    }
    if b.divider_mode == DividerMode::Grid {
        name += "_rasterfach";
    }
    if b.label_tab {
        name += "_label";
        if b.label_side != Side::Back {
            name.push(b.label_side.letter());
        }
    }
    if b.scoop > 0 {
        name += &format!("_mulde{}", b.scoop);
        let sides = b.scoops();
        if sides != [Side::Front] {
            name.extend(sides.iter().map(|s| s.letter()));
        }
    }
    if b.open_front {
        name += "_offen";
    }
    if b.finger_cutouts && b.dividers != [1, 1] {
        name += "_finger";
    }
    if let Some(w) = &b.window {
        name += &format!("_fenster{}", mm(w.width));
    }
    if !b.cutouts.is_empty() {
        name += &format!("_{}ausschnitt", b.cutouts.len());
    }
    if let Some(text) = b.label_text.as_deref().map(slug).filter(|s| !s.is_empty()) {
        name += &format!("_{text}");
    }
    if !b.lip {
        name += "_ohne_rand";
    }
    // A 1x1 bin has only corner holes, so "corners" and "all" are the same part there.
    match b.magnets {
        MagnetMode::None => {}
        MagnetMode::All => name += "_magnete",
        MagnetMode::Corners if gx * gy == 1 => name += "_magnete",
        MagnetMode::Corners => name += "_magnete_ecken",
    }
    name + ".stl"
}

pub fn lid(l: &LidSpec) -> String {
    let [gx, gy] = l.cells;
    format!("deckel_{gx}x{gy}{}.stl", if l.clicks { "_klick" } else { "" })
}

pub fn holder(h: &HolderSpec) -> String {
    match h {
        HolderSpec::Tape(t) => tape(t),
        _ => {
            let [gx, gy] = h.cells();
            format!("halter_{}_{gx}x{gy}_{}u.stl", h.slug(), h.units())
        }
    }
}

pub fn tape(t: &TapeSpec) -> String {
    let [gx, gy] = t.cells();
    let n = t.stacks[0] * t.stacks[1];
    let places = if n > 1 { "plaetze" } else { "platz" };
    format!("tape_{gx}x{gy}_{n}{places}_{}hoch_bis{}mm.stl", t.height, t.max_roll().floor() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mount;

    #[test]
    fn names_match_the_python_script() {
        let build = |floor: PlateFloor, mount: Mount| PlateBuild { floor, mount, ..PlateBuild::default() };
        assert_eq!(baseplate(0, 1, [2, 4], build(PlateFloor::Open, Mount::default())), "baseplate_1B_2x4.stl");
        assert_eq!(
            baseplate(1, 0, [5, 3], build(PlateFloor::Magnets, Mount::default())),
            "baseplate_2A_5x3_magnete.stl"
        );
        let mount = Mount { screws: true, connectors: true, ..Mount::default() };
        assert_eq!(
            baseplate(0, 0, [5, 3], build(PlateFloor::Solid, mount)),
            "baseplate_1A_5x3_boden_schrauben_verbinder.stl"
        );
        let full =
            PlateBuild { style: PlateStyle::Full, magnets_below: true, ..build(PlateFloor::Solid, Mount::default()) };
        assert_eq!(baseplate(0, 0, [5, 3], full), "baseplate_1A_5x3_boden_voll_magnete_unten.stl");
        let b = BinSpec { cells: [2, 1], units: 9, dividers: [2, 1], ..BinSpec::default() };
        assert_eq!(bin(&b), "bin_2x1_9u_2x1fach.stl");
        let b = BinSpec { cells: [1, 1], magnets: MagnetMode::Corners, lip: false, ..BinSpec::default() };
        assert_eq!(bin(&b), "bin_1x1_6u_ohne_rand_magnete.stl");
        let wall = PlateBuild { wall: true, hooks: true, style: PlateStyle::Full, ..build(PlateFloor::Magnets, mount) };
        assert_eq!(baseplate(0, 0, [8, 6], wall), "wandplatte_1A_8x6_magnete_haken_schrauben_verbinder.stl");
        let b = BinSpec { cells: [2, 1], units: 6, dividers: [3, 1], open_front: true, ..BinSpec::default() };
        assert_eq!(wall_bin(&b, true), "wandbin_2x1_6u_3x1fach_offen.stl");
        assert_eq!(wall_bin(&BinSpec::default(), false), "wandbin_1x1_6u_ohne_haken.stl");
        let t = TapeSpec { od: 103.0, stacks: [2, 1], height: 3, ..TapeSpec::default() };
        assert_eq!(tape(&t), "tape_5x3_2plaetze_3hoch_bis103mm.stl");
        let t = TapeSpec { od: 124.0, stacks: [1, 1], height: 2, ..TapeSpec::default() };
        assert_eq!(tape(&t), "tape_3x3_1platz_2hoch_bis124mm.stl");
    }
}
