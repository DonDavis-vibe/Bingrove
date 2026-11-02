//! Stroke lettering with the Hershey Simplex font, for engraving labels, nut sizes and
//! calibration marks. The glyphs are polylines; [`text_2d`] thickens them into polygons, the
//! rest extrudes those or cuts them into a face.
//!
//! Lengths are mm. A text starts at the origin on its baseline and runs along +x; `height` is
//! the height of the capitals, descenders reach about a third of it below the baseline.
//!
//! The glyph data is `futural.jhf` of <https://github.com/kamalmostafa/hershey-fonts>, used under
//! the Hershey use restriction, which asks for this acknowledgement to travel with the data:
//!
//! - The Hershey Fonts were originally created by Dr. A. V. Hershey while working at the U. S.
//!   National Bureau of Standards.
//! - The format of the Font data in this distribution was originally created by James Hurt,
//!   Cognition, Inc., 900 Technology Park Drive, Billerica, MA 01821 (mit-eddie!ci-dandelion!hurt).

use std::sync::OnceLock;

use manifold_rust::cross_section::CrossSection;

use super::kernel::{Solid, Vec2, Vec3};

/// The 95 glyphs of ASCII 32..=126, one per line: glyph number, vertex count, then coordinate
/// pairs as characters relative to 'R'. The first pair is the left and right bound, " R" lifts
/// the pen. (The file carries one more glyph after '~', which is not used.)
const FONT_DATA: &str = include_str!("hershey_simplex.jhf");
/// Printable ASCII, the characters the font covers.
const GLYPHS: usize = 95;
/// The baseline's row below 'R' in the font data; rows count downwards.
const BASELINE: i32 = 9;
/// Rows from the baseline to the top of the capitals.
const CAP_HEIGHT: f64 = 21.0;
/// Corners of the disc at every vertex of a stroke.
const JOINT_SEGMENTS: usize = 12;
/// How far the engraving body sticks out above the face it is cut into.
const OVERSHOOT: f64 = 0.5;

/// Default stroke width as a fraction of the cap height; see [`stroke_for`].
pub const STROKE: f64 = 0.15;
/// Narrowest default stroke: thinner lines do not print or engrave cleanly.
pub const MIN_STROKE: f64 = 0.5;

/// The default stroke width for a text of `height`: [`STROKE`] × height, at least [`MIN_STROKE`].
pub fn stroke_for(height: f64) -> f64 {
    (STROKE * height).max(MIN_STROKE)
}

struct Glyph {
    /// Pen advance in font units.
    advance: f64,
    /// Polylines in font units: x from the glyph's left bound, y up from the baseline.
    strokes: Vec<Vec<[f64; 2]>>,
}

fn font() -> &'static [Glyph] {
    static FONT: OnceLock<Vec<Glyph>> = OnceLock::new();
    FONT.get_or_init(|| {
        FONT_DATA
            .lines()
            .take(GLYPHS)
            .map(|line| parse_glyph(line).expect("hershey_simplex.jhf is embedded and well formed"))
            .collect()
    })
}

fn parse_glyph(line: &str) -> Option<Glyph> {
    let bytes = line.as_bytes();
    let n: usize = line.get(5..8)?.trim().parse().ok()?;
    let coords = bytes.get(8..8 + 2 * n)?;
    let (left, right) = (coords.first()?, coords.get(1)?);
    let unit = |c: u8| c as i32 - b'R' as i32;
    let mut strokes: Vec<Vec<[f64; 2]>> = vec![Vec::new()];
    for pair in coords[2..].as_chunks::<2>().0 {
        if pair == b" R" {
            if !strokes.last().is_some_and(Vec::is_empty) {
                strokes.push(Vec::new());
            }
            continue;
        }
        let x = unit(pair[0]) - unit(*left);
        let y = BASELINE - unit(pair[1]);
        strokes.last_mut()?.push([x as f64, y as f64]);
    }
    strokes.retain(|s| !s.is_empty());
    Some(Glyph { advance: (unit(*right) - unit(*left)) as f64, strokes })
}

/// The glyph of `c`; characters outside printable ASCII are drawn as a space.
fn glyph(c: char) -> &'static Glyph {
    let i = (c as u32).checked_sub(32).filter(|&i| (i as usize) < GLYPHS).unwrap_or(0);
    &font()[i as usize]
}

/// Advance width of `text` at cap height `height`: the pen's travel, without the half stroke
/// that the first and last glyph's ink may stick out.
pub fn text_width(text: &str, height: f64) -> f64 {
    text.chars().map(|c| glyph(c).advance).sum::<f64>() * height / CAP_HEIGHT
}

/// A disc of radius `r` around `c`, counter-clockwise.
fn disc(c: Vec2, r: f64) -> Vec<Vec2> {
    (0..JOINT_SEGMENTS)
        .map(|i| {
            let a = (i as f64 + 0.5) * std::f64::consts::TAU / JOINT_SEGMENTS as f64;
            Vec2::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

/// The rectangle of half width `hw` along the segment `a`–`b`, counter-clockwise; `None` for a
/// segment too short to have a direction.
fn bar(a: Vec2, b: Vec2, hw: f64) -> Option<Vec<Vec2>> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        return None;
    }
    let n = Vec2::new(-dy / len * hw, dx / len * hw);
    Some(vec![
        Vec2::new(a.x - n.x, a.y - n.y),
        Vec2::new(b.x - n.x, b.y - n.y),
        Vec2::new(b.x + n.x, b.y + n.y),
        Vec2::new(a.x + n.x, a.y + n.y),
    ])
}

/// The strokes of `text` as filled polygons `stroke` wide with round joints and caps, united
/// into polygons with holes (outlines counter-clockwise, holes clockwise), as `Solid::extrude`
/// and `polygon_prism` expect. Empty for an empty text.
pub fn text_2d(text: &str, height: f64, stroke: f64) -> Vec<Vec<Vec2>> {
    if !(height > 0.0 && stroke > 0.0) {
        return Vec::new();
    }
    let scale = height / CAP_HEIGHT;
    let hw = stroke / 2.0;
    let mut pieces: Vec<Vec<Vec2>> = Vec::new();
    let mut x = 0.0;
    for c in text.chars() {
        let g = glyph(c);
        for stroke in &g.strokes {
            let pts: Vec<Vec2> = stroke.iter().map(|p| Vec2::new(x + p[0] * scale, p[1] * scale)).collect();
            pieces.extend(pts.iter().map(|&p| disc(p, hw)));
            pieces.extend(pts.windows(2).filter_map(|w| bar(w[0], w[1], hw)));
        }
        x += g.advance * scale;
    }
    if pieces.is_empty() {
        return Vec::new();
    }
    // The constructor unions its contours with the positive fill rule, so the overlapping
    // bars and discs merge into clean outlines.
    CrossSection::new(pieces).to_polygons()
}

/// `text` as a solid standing on z = 0, `depth` tall. Empty for a text without ink.
pub fn text_solid(text: &str, height: f64, stroke: f64, depth: f64) -> Solid {
    let polys = text_2d(text, height, stroke);
    if polys.is_empty() || depth <= 0.0 {
        return Solid::empty();
    }
    Solid::extrude(&polys, depth, 0, 0.0, Vec2::new(1.0, 1.0))
}

/// `text` cut `depth` deep into the face z = `at[2]` of `solid`, centred on `at` (x, y) by its
/// advance width and cap height, in the default stroke of its height.
pub fn engrave(solid: Solid, text: &str, at: [f64; 3], height: f64, depth: f64) -> Solid {
    engrave_turned(solid, text, at, height, depth, 0)
}

/// The cap height at which `text` is at most `max_width` wide, starting from `height`.
pub fn fit_height(text: &str, height: f64, max_width: f64) -> f64 {
    let width = text_width(text, height);
    if width > max_width && width > 0.0 { height * max_width / width } else { height }
}

/// `text` cut `depth` deep into the face y = `at[1]` of `solid` that looks towards −y (a bin's
/// front wall), centred on `at` (x, z) and read from the front.
pub fn engrave_front(solid: Solid, text: &str, at: [f64; 3], height: f64, depth: f64) -> Solid {
    let body = text_solid(text, height, stroke_for(height), depth + OVERSHOOT);
    if body.is_empty() {
        return solid;
    }
    // Lying in the x-y plane, the text is stood up: its y becomes z, its thickness runs along −y.
    let body = body
        .translate(Vec3::new(-text_width(text, height) / 2.0, -height / 2.0, 0.0))
        .rotate(90.0, 0.0, 0.0)
        .translate(Vec3::new(at[0], at[1] + depth, at[2]));
    solid - body
}

/// [`engrave`] with the text turned counter-clockwise about `at` by `quarter_turns` × 90°.
pub fn engrave_turned(solid: Solid, text: &str, at: [f64; 3], height: f64, depth: f64, quarter_turns: u32) -> Solid {
    let body = text_solid(text, height, stroke_for(height), depth + OVERSHOOT);
    if body.is_empty() {
        return solid;
    }
    let body = body
        .translate(Vec3::new(-text_width(text, height) / 2.0, -height / 2.0, -depth))
        .rotate(0.0, 0.0, 90.0 * (quarter_turns % 4) as f64)
        .translate(Vec3::new(at[0], at[1], at[2]));
    solid - body
}

#[cfg(test)]
mod tests {
    use super::super::kernel::{cuboid, finish};
    use super::*;

    #[test]
    fn every_printable_ascii_character_has_a_glyph() {
        assert_eq!(font().len(), GLYPHS);
        assert!(glyph(' ').strokes.is_empty() && glyph(' ').advance > 0.0);
        assert_eq!(glyph('I').strokes.len(), 1);
        assert_eq!(glyph('A').strokes.len(), 3);
        assert!(glyph('~').advance > 0.0);
        // Unknown characters are spaces.
        assert_eq!(glyph('ä').advance, glyph(' ').advance);
        assert!(glyph('ä').strokes.is_empty());
    }

    #[test]
    fn capitals_span_the_height_and_m_is_wider_than_i() {
        assert!(text_width("M", 5.0) > text_width("I", 5.0));
        assert!((text_width("II", 5.0) - 2.0 * text_width("I", 5.0)).abs() < 1e-9);
        assert_eq!(text_width("", 5.0), 0.0);
        let top = glyph('H').strokes.iter().flatten().map(|p| p[1]).fold(f64::MIN, f64::max);
        let bottom = glyph('H').strokes.iter().flatten().map(|p| p[1]).fold(f64::MAX, f64::min);
        assert_eq!((bottom, top), (0.0, CAP_HEIGHT));
    }

    #[test]
    fn an_eight_is_a_closed_solid_of_plausible_volume() {
        let depth = 1.0;
        let s = text_solid("8", 6.0, 0.8, depth);
        let mesh = finish(&s, "8").unwrap();
        assert!(mesh.is_closed());
        let (lo, hi) = mesh.bbox();
        let area = (hi[0] - lo[0]) as f64 * (hi[1] - lo[1]) as f64;
        let v = s.volume();
        assert!(v > 0.3 * area * depth && v < area * depth, "volume {v} for a bbox area of {area}");
        assert!(lo[2].abs() < 1e-4 && (hi[2] as f64 - depth).abs() < 1e-4);
        // The skeleton spans the cap height, the ink half a stroke more on each side.
        assert!((hi[1] as f64 - lo[1] as f64 - 6.8).abs() < 0.05);
    }

    #[test]
    fn empty_text_has_no_solid() {
        assert!(text_2d("", 5.0, 0.8).is_empty());
        assert!(text_solid("   ", 5.0, 0.8, 1.0).is_empty());
        let block = cuboid([20.0, 10.0, 3.0], [0.0; 3]);
        let v = block.volume();
        assert_eq!(engrave(block, " ", [10.0, 5.0, 3.0], 5.0, 0.6).volume(), v);
    }

    #[test]
    fn engraving_removes_material_from_the_top_face() {
        let block = cuboid([20.0, 10.0, 3.0], [0.0; 3]);
        let before = block.volume();
        let engraved = engrave(block.clone(), "13", [10.0, 5.0, 3.0], 5.0, 0.6);
        let after = engraved.volume();
        assert!(after < before - 1.0 && after > before - 20.0 * 10.0 * 0.6, "{before} -> {after}");
        assert!(finish(&engraved, "engraved").unwrap().is_closed());
        let bb = engraved.bounding_box();
        assert!((bb.max.z - 3.0).abs() < 1e-6 && bb.min.z.abs() < 1e-6, "the cut stays inside the block");
        // A quarter turn moves the same ink into the other direction: equal volume, upright in y.
        let turned = engrave_turned(block, "13", [10.0, 5.0, 3.0], 5.0, 0.6, 1);
        assert!((turned.volume() - after).abs() < 1e-3 * before);
        let tall = cuboid([8.0, 20.0, 3.0], [0.0; 3]);
        let v = tall.volume();
        let cut = v - engrave_turned(tall, "13", [4.0, 10.0, 3.0], 5.0, 0.6, 1).volume();
        assert!((cut - (before - after)).abs() < 1e-3 * v, "the turned text fits the tall block");
    }

    #[test]
    fn eight_characters_render_quickly() {
        let start = std::time::Instant::now();
        let s = text_solid("1/4 M3.5", 5.0, 0.8, 1.0);
        let elapsed = start.elapsed();
        eprintln!("text_solid of 8 characters: {elapsed:?}");
        assert!(!s.is_empty() && finish(&s, "label").unwrap().is_closed());
        assert!(elapsed.as_millis() < 100, "took {elapsed:?}");
    }
}
