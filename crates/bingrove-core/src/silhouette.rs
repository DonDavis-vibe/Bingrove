//! The outline of a thing seen from above, from its triangles: the convex hull of the points
//! projected onto the floor. Good enough for most objects a pocket is cut for; concave outlines
//! get their hollows filled, which only costs some room.

/// Which way is down when the thing lies in the pocket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Down {
    /// As modelled (z down).
    #[default]
    Z,
    /// Lying on its side: the model's x points down.
    X,
    /// The model's y points down.
    Y,
}

/// Outline and height of a mesh (flat `positions`, three floats per vertex) with `down` pointing
/// into the pocket. The outline is counter-clockwise with its minimum corner at the origin.
pub fn outline(positions: &[f32], down: Down) -> Option<(Vec<[f64; 2]>, f64)> {
    let pts: Vec<[f64; 3]> =
        positions.as_chunks::<3>().0.iter().map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]).collect();
    if pts.len() < 3 {
        return None;
    }
    let (u, v, w) = match down {
        Down::Z => (0, 1, 2),
        Down::X => (1, 2, 0),
        Down::Y => (2, 0, 1),
    };
    let flat: Vec<[f64; 2]> = pts.iter().map(|p| [p[u], p[v]]).collect();
    let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p[w]), hi.max(p[w])));
    let mut hull = convex_hull(flat);
    if hull.len() < 3 {
        return None;
    }
    let (x0, y0) = hull.iter().fold((f64::INFINITY, f64::INFINITY), |(x, y), p| (x.min(p[0]), y.min(p[1])));
    for p in &mut hull {
        p[0] -= x0;
        p[1] -= y0;
    }
    Some((hull, hi - lo))
}

/// Andrew's monotone chain; counter-clockwise, no repeated last point.
pub fn convex_hull(mut pts: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut lower: Vec<[f64; 2]> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<[f64; 2]> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Size of an outline.
pub fn size(outline: &[[f64; 2]]) -> [f64; 2] {
    let (w, d) = outline.iter().fold((0.0f64, 0.0f64), |(w, d), p| (w.max(p[0]), d.max(p[1])));
    [w, d]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hull_of_a_box_is_its_rectangle() {
        // A 30 x 20 x 10 box as a triangle soup, with a point inside that must not show up.
        let mut pos = Vec::new();
        for x in [0.0, 30.0] {
            for y in [0.0, 20.0] {
                for z in [0.0, 10.0] {
                    pos.extend([x as f32, y as f32, z as f32]);
                }
            }
        }
        pos.extend([15.0, 10.0, 5.0]);
        let (hull, h) = outline(&pos, Down::Z).unwrap();
        assert_eq!(hull.len(), 4);
        assert_eq!(size(&hull), [30.0, 20.0]);
        assert_eq!(h, 10.0);
        // On its side the box is 20 x 10 in the floor and 30 tall.
        let (hull, h) = outline(&pos, Down::X).unwrap();
        assert_eq!(size(&hull), [20.0, 10.0]);
        assert_eq!(h, 30.0);
        // Counter-clockwise: positive area.
        let area: f64 = hull.iter().zip(hull.iter().cycle().skip(1)).map(|(a, b)| a[0] * b[1] - b[0] * a[1]).sum();
        assert!(area > 0.0);
    }
}
