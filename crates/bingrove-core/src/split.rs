//! Cutting the plate into pieces that fit the printer: cuts only on grid lines, as few pieces as
//! possible, the largest piece as small as possible, larger pieces first (front, left).

use crate::grid::Grid;
use crate::model::BedSpec;
use crate::spec::GRID;

/// Sizes of the pieces along one axis for a given split; `reach` is how far the plate extends
/// past the outer grid line on each side (left, right, front, back).
pub fn extents(reach: [f64; 4], axis: usize, split: &[u32]) -> Vec<f64> {
    let (lo, hi) = (reach[2 * axis], reach[2 * axis + 1]);
    let last = split.len().saturating_sub(1);
    split
        .iter()
        .enumerate()
        .map(|(i, &c)| c as f64 * GRID + if i == 0 { lo } else { 0.0 } + if i == last { hi } else { 0.0 })
        .collect()
}

/// Most cells a piece may have when it may be at most `max` long.
fn cap(max: f64, extra: f64) -> u32 {
    ((max - extra) / GRID + 1e-9).floor().max(0.0) as u32
}

/// Caps of `k` pieces: first, middle ones, last.
fn caps(k: usize, max: f64, lo: f64, hi: f64) -> Vec<u32> {
    if k == 1 {
        return vec![cap(max, lo + hi)];
    }
    (0..k).map(|i| cap(max, if i == 0 { lo } else { 0.0 } + if i == k - 1 { hi } else { 0.0 })).collect()
}

fn feasible(n: u32, caps: &[u32]) -> bool {
    caps.iter().all(|&c| c >= 1) && n >= caps.len() as u32 && n <= caps.iter().sum::<u32>()
}

/// Split of `n` cells into `k` pieces with the smallest largest piece, larger pieces first.
fn best(n: u32, k: usize, lo: f64, hi: f64) -> Option<(Vec<u32>, f64)> {
    if k == 0 || k as u32 > n {
        return None;
    }
    // Candidate maxima are the possible piece sizes; take the smallest that works.
    let mut candidates: Vec<f64> = (1..=n).flat_map(|c| [0.0, lo, hi, lo + hi].map(|e| c as f64 * GRID + e)).collect();
    candidates.sort_by(f64::total_cmp);
    let max = candidates.into_iter().find(|&m| feasible(n, &caps(k, m, lo, hi)))?;
    let caps = caps(k, max, lo, hi);
    let mut split = Vec::with_capacity(k);
    let mut left = n;
    for i in 0..k {
        let rest = &caps[i + 1..];
        let take = (1..=caps[i].min(left)).rev().find(|&c| rest.is_empty() && c == left || feasible(left - c, rest))?;
        split.push(take);
        left -= take;
    }
    Some((split, max))
}

/// The split of the whole plate for `bed`. `None` when even single cells do not fit.
pub fn split_plate(grid: &Grid, reach: [f64; 4], bed: &BedSpec) -> Option<[Vec<u32>; 2]> {
    let [nx, ny] = grid.cells;
    let [lx, hx, ly, hy] = reach;
    let mut found: Option<(u32, f64, [Vec<u32>; 2])> = None;
    for kx in 1..=nx as usize {
        let Some((sx, mx)) = best(nx, kx, lx, hx) else { continue };
        for ky in 1..=ny as usize {
            let Some((sy, my)) = best(ny, ky, ly, hy) else { continue };
            if !bed.fits(mx, my) {
                continue;
            }
            let pieces = (kx * ky) as u32;
            let largest = mx.max(my);
            if found.as_ref().is_none_or(|(p, l, _)| pieces < *p || pieces == *p && largest < *l - 1e-9) {
                found = Some((pieces, largest, [sx.clone(), sy]));
            }
            break;
        }
    }
    found.map(|(_, _, s)| s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PlateOptions, PlateSpec};
    use crate::presets::Preset;
    use crate::profile::MagnetSpec;

    fn grid(drawer: [f64; 2], o: &PlateOptions) -> Grid {
        Grid::new(drawer, o, &MagnetSpec::default()).unwrap().0
    }

    #[test]
    fn python_splits_with_solid_margins() {
        for p in Preset::all() {
            let PlateSpec { drawer, split, .. } = p.plate.clone();
            let mut o = p.plate.options();
            o.split = None;
            assert_eq!(
                {
                    let g = grid(drawer, &o);
                    split_plate(&g, g.reach(), &BedSpec::default())
                }
                .unwrap(),
                split,
                "{}",
                p.name
            );
        }
    }

    #[test]
    fn cut_back_edges_need_fewer_pieces() {
        // ToFloor edge zones left and right: the plate is 212 mm wide and fits in one piece.
        let g = grid([250.0, 305.0], &PlateOptions::default());
        assert_eq!(split_plate(&g, g.reach(), &BedSpec::default()).unwrap(), [vec![5], vec![4, 3]]);
        assert_eq!(extents(g.reach(), 0, &[5]), [212.0]);
        assert_eq!(extents(g.reach(), 1, &[4, 3]), [173.0, 131.0]);
    }

    #[test]
    fn rectangular_bed_turns_pieces() {
        let g = grid([300.0, 200.0], &PlateOptions { edge_threshold: f64::INFINITY, ..PlateOptions::default() });
        let bed = BedSpec { x: 180.0, y: 300.0, margin: 0.0, ..BedSpec::default() };
        // 299 x 199: lying along the long side of the bed it would fit, but 199 > 180 across.
        let s = split_plate(&g, g.reach(), &bed).unwrap();
        assert_eq!(s.iter().map(Vec::len).product::<usize>(), 2);
    }
}
