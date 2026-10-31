//! The few solid-modelling calls Bingrove needs, on top of manifold-rust. Keeping them in one
//! place keeps the kernel swappable.

use std::f64::consts::FRAC_PI_2;

pub use manifold_rust::linalg::{Vec2, Vec3};
pub use manifold_rust::manifold::Manifold as Solid;
use manifold_rust::types::{Error, OpType};

use crate::mesh::TriMesh;

/// Rounded rectangle centred on the origin, counter-clockwise, each corner an arc of `seg / 4`
/// segments. Radii below 0.2 mm are raised to 0.2 so insets never produce a sharp corner.
pub fn rrect(w: f64, h: f64, r: f64, seg: u32) -> Vec<Vec2> {
    let r = r.max(0.2);
    let (hx, hy) = (w / 2.0 - r, h / 2.0 - r);
    let n = (seg / 4).max(1);
    let mut pts: Vec<Vec2> = Vec::with_capacity(4 * (n as usize + 1));
    for (k, (cx, cy)) in [(hx, hy), (-hx, hy), (-hx, -hy), (hx, -hy)].into_iter().enumerate() {
        for i in 0..=n {
            let a = (k as f64 + i as f64 / n as f64) * FRAC_PI_2;
            let p = Vec2::new(cx + r * a.cos(), cy + r * a.sin());
            // A side of zero length (w or h equal to 2r) would repeat the corner point.
            if pts.last().is_none_or(|q| (q.x - p.x).abs() > 1e-9 || (q.y - p.y).abs() > 1e-9) {
                pts.push(p);
            }
        }
    }
    if pts.len() > 1 {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        if (a.x - b.x).abs() <= 1e-9 && (a.y - b.y).abs() <= 1e-9 {
            pts.pop();
        }
    }
    pts
}

fn ring(w: f64, h: f64, r: f64, z: f64, seg: u32) -> impl Iterator<Item = Vec3> {
    rrect(w, h, r, seg).into_iter().map(move |p| Vec3::new(p.x, p.y, z))
}

/// Prism of a rounded rectangle from z = 0 to `height`.
pub fn rrect_prism(w: f64, h: f64, r: f64, height: f64, seg: u32) -> Solid {
    Solid::extrude(&vec![rrect(w, h, r, seg)], height, 0, 0.0, Vec2::new(1.0, 1.0))
}

/// A rounded rectangle swept along a `(z, inset)` profile: each pair of neighbouring profile
/// points becomes the hull of two rings, and the rings are united. Centred on `at`.
pub fn swept(w: f64, h: f64, r: f64, profile: &[(f64, f64)], at: (f64, f64), seg: u32) -> Solid {
    let parts: Vec<Solid> = profile
        .windows(2)
        .map(|pair| {
            let ((z0, d0), (z1, d1)) = (pair[0], pair[1]);
            let pts: Vec<Vec3> = ring(w - 2.0 * d0, h - 2.0 * d0, r - d0, z0, seg)
                .chain(ring(w - 2.0 * d1, h - 2.0 * d1, r - d1, z1, seg))
                .collect();
            Solid::hull(&pts)
        })
        .collect();
    union(parts).translate(Vec3::new(at.0, at.1, 0.0))
}

/// A prism over a polygon (counter-clockwise) grown by `offset` with round joins, from z = 0 to `height`.
pub fn polygon_prism(points: &[[f64; 2]], offset: f64, height: f64, seg: u32) -> Solid {
    use manifold_rust::cross_section::CrossSection;
    let pts: Vec<Vec2> = points.iter().map(|p| Vec2::new(p[0], p[1])).collect();
    let section = CrossSection::new(vec![pts]);
    let polys = if offset.abs() > 1e-9 {
        section.offset_with_params(offset, 1, 2.0, seg as i32).to_polygons()
    } else {
        section.to_polygons()
    };
    Solid::extrude(&polys, height, 0, 0.0, Vec2::new(1.0, 1.0))
}

/// A hexagonal prism with `across_flats`, one pair of flats along x, standing on z = 0.
pub fn hex_prism(across_flats: f64, height: f64) -> Solid {
    let r = across_flats / 3f64.sqrt();
    let pts: Vec<Vec2> = (0..6)
        .map(|k| {
            let a = (k as f64 + 0.5) * std::f64::consts::FRAC_PI_3;
            Vec2::new(r * a.cos(), r * a.sin())
        })
        .collect();
    Solid::extrude(&vec![pts], height, 0, 0.0, Vec2::new(1.0, 1.0))
}

/// Digits (and '.') as seven-segment strokes, `height` mm tall, extruded by `depth`; the text
/// starts at the origin and runs along x. For engraving labels without a font.
pub fn digits(text: &str, height: f64, depth: f64) -> Solid {
    let (h, w, t) = (height, height * 0.55, height * 0.16);
    // Segments a..g as boxes within a w × h cell: (x, y, width, height).
    let seg = |s: char| -> [f64; 4] {
        match s {
            'a' => [0.0, h - t, w, t],
            'b' => [w - t, h / 2.0, t, h / 2.0],
            'c' => [w - t, 0.0, t, h / 2.0],
            'd' => [0.0, 0.0, w, t],
            'e' => [0.0, 0.0, t, h / 2.0],
            'f' => [0.0, h / 2.0, t, h / 2.0],
            _ => [0.0, h / 2.0 - t / 2.0, w, t],
        }
    };
    let on = |c: char| -> &'static str {
        match c {
            '0' => "abcdef",
            '1' => "bc",
            '2' => "abged",
            '3' => "abgcd",
            '4' => "fgbc",
            '5' => "afgcd",
            '6' => "afgedc",
            '7' => "abc",
            '8' => "abcdefg",
            '9' => "abcdfg",
            _ => "",
        }
    };
    let mut parts = Vec::new();
    let mut x = 0.0;
    for c in text.chars() {
        if c == '.' {
            parts.push(cuboid([t, t, depth], [x, 0.0, 0.0]));
            x += t * 2.0;
            continue;
        }
        for s in on(c).chars() {
            let [sx, sy, sw, sh] = seg(s);
            parts.push(cuboid([sw, sh, depth], [x + sx, sy, 0.0]));
        }
        x += w + t * 1.5;
    }
    union(parts)
}

/// Axis-aligned box with its minimum corner at `at`.
pub fn cuboid(size: [f64; 3], at: [f64; 3]) -> Solid {
    Solid::cube(Vec3::new(size[0], size[1], size[2]), false).translate(Vec3::new(at[0], at[1], at[2]))
}

/// Axis-aligned box centred on `at`.
pub fn cuboid_centred(size: [f64; 3], at: [f64; 3]) -> Solid {
    Solid::cube(Vec3::new(size[0], size[1], size[2]), true).translate(Vec3::new(at[0], at[1], at[2]))
}

/// Cylinder or cone standing on z = 0.
pub fn cylinder(height: f64, r_low: f64, r_high: f64, seg: u32) -> Solid {
    Solid::cylinder(height, r_low, r_high, seg as i32)
}

pub fn union(parts: Vec<Solid>) -> Solid {
    match parts.len() {
        0 => Solid::empty(),
        1 => parts.into_iter().next().unwrap(),
        _ => Solid::batch_boolean(&parts, OpType::Add),
    }
}

/// `base` minus every one of `cuts`.
pub fn subtract(base: Solid, cuts: Vec<Solid>) -> Solid {
    if cuts.is_empty() { base } else { base - union(cuts) }
}

pub fn translate(s: &Solid, v: [f64; 3]) -> Solid {
    s.translate(Vec3::new(v[0], v[1], v[2]))
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GeomError {
    #[error("the solid for {0} is broken ({1:?})")]
    Kernel(String, Error),
    #[error("the solid for {0} came out empty")]
    Empty(String),
    #[error("{0}")]
    Invalid(String),
}

/// Checks the kernel status and turns the solid into a triangle mesh.
pub fn finish(s: &Solid, what: &str) -> Result<TriMesh, GeomError> {
    match s.status() {
        Error::NoError if s.is_empty() => Err(GeomError::Empty(what.into())),
        Error::NoError => Ok(TriMesh::from_solid(s)),
        e => Err(GeomError::Kernel(what.into(), e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_rectangle_has_the_expected_area() {
        let (w, h, r) = (41.5, 83.5, 3.75);
        let s = rrect_prism(w, h, r, 1.0, 256);
        let exact = w * h - (4.0 - std::f64::consts::PI) * r * r;
        assert!((s.volume() - exact).abs() < 0.05, "{} vs {exact}", s.volume());
    }

    #[test]
    fn square_with_full_radius_has_no_repeated_points() {
        let pts = rrect(8.0, 8.0, 4.0, 64);
        assert_eq!(pts.len(), 4 * 16);
    }

    #[test]
    fn swept_profile_is_a_closed_solid() {
        let foot = swept(41.5, 41.5, 3.75, &crate::spec::FOOT_PROFILE, (0.0, 0.0), 64);
        let mesh = finish(&foot, "foot").unwrap();
        assert!(mesh.is_closed());
        let bb = mesh.bbox();
        assert!((bb.1[2] - 4.75).abs() < 1e-4 && (bb.1[0] - bb.0[0] - 41.5).abs() < 1e-4);
    }
}
