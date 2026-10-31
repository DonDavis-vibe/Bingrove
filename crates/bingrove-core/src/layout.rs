//! Placements on the grid: exact-cover check, repairing layouts that came from elsewhere (an AI,
//! an old project file) and the parts list (one entry per distinct bin).

use serde::{Deserialize, Serialize};

use crate::model::{BinSpec, Placement};
use crate::solver::{Heights, Occupancy, fill};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LayoutError {
    #[error("bin {w}x{d} at cell ({x}, {y}) reaches past the {nx} x {ny} grid")]
    OutOfGrid { x: u32, y: u32, w: u32, d: u32, nx: u32, ny: u32 },
    /// Cells covered by no bin or by more than one; `counts[y][x]`, row 0 at the front.
    #[error("the bins do not cover the grid exactly once:\n{}", show(counts))]
    NotExact { counts: Vec<Vec<u32>> },
}

/// Bins per cell, back row first, as the drawer looks from above with the front at the bottom.
fn show(counts: &[Vec<u32>]) -> String {
    counts
        .iter()
        .rev()
        .map(|row| row.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Checks that the placements fill an `nx` × `ny` grid exactly once.
pub fn validate(nx: u32, ny: u32, placements: &[Placement]) -> Result<(), LayoutError> {
    let mut counts = vec![vec![0u32; nx as usize]; ny as usize];
    for p in placements {
        let [w, d] = p.cells();
        if p.x + w > nx || p.y + d > ny {
            return Err(LayoutError::OutOfGrid { x: p.x, y: p.y, w, d, nx, ny });
        }
        for row in &mut counts[p.y as usize..(p.y + d) as usize] {
            for c in &mut row[p.x as usize..(p.x + w) as usize] {
                *c += 1;
            }
        }
    }
    if counts.iter().flatten().any(|&c| c != 1) {
        return Err(LayoutError::NotExact { counts });
    }
    Ok(())
}

/// What `repair` changed, so whoever sent the layout can see it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub enum Repair {
    Moved { index: usize, from: [u32; 2], to: [u32; 2] },
    Dropped { index: usize, cells: [u32; 2] },
    Filled { count: usize },
}

/// Makes any list of placements valid: keeps them in the given order, moves one that collides or
/// sticks out to the nearest free spot of its size, drops it when there is none, then fills
/// the remaining cells with bins.
pub fn repair(nx: u32, ny: u32, placements: Vec<Placement>, h: Heights) -> (Vec<Placement>, Vec<Repair>) {
    let mut occ = Occupancy::new(nx, ny);
    let mut out = Vec::with_capacity(placements.len());
    let mut repairs = Vec::new();
    for (index, mut p) in placements.into_iter().enumerate() {
        let cells = p.cells();
        if p.holder.is_some() {
            p.bin.cells = cells;
        }
        let from = [p.x, p.y];
        let spot = if occ.free(p.x, p.y, cells) {
            Some((p.x, p.y))
        } else {
            // Nearest by Manhattan distance; ties go to the front, then to the left.
            let mut spots: Vec<(u32, u32)> =
                (0..ny).flat_map(|y| (0..nx).map(move |x| (x, y))).filter(|&(x, y)| occ.free(x, y, cells)).collect();
            spots.sort_by_key(|&(x, y)| (x.abs_diff(p.x) + y.abs_diff(p.y), y, x));
            spots.first().copied()
        };
        match spot {
            Some((x, y)) => {
                occ.take(x, y, cells);
                if [x, y] != from {
                    repairs.push(Repair::Moved { index, from, to: [x, y] });
                }
                p.x = x;
                p.y = y;
                out.push(p);
            }
            None => repairs.push(Repair::Dropped { index, cells }),
        }
    }
    let filled = fill(&mut occ, h);
    if !filled.is_empty() {
        repairs.push(Repair::Filled { count: filled.len() });
    }
    out.extend(filled);
    (out, repairs)
}

/// Distinct bins with how often each is needed, in order of first appearance.
pub fn bill_of_bins(placements: &[Placement]) -> Vec<(BinSpec, u32)> {
    let mut out: Vec<(BinSpec, u32)> = Vec::new();
    for p in placements {
        match out.iter_mut().find(|(b, _)| *b == p.bin) {
            Some((_, n)) => *n += 1,
            None => out.push((p.bin.clone(), 1)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::Preset;

    fn bin(x: u32, y: u32, w: u32, d: u32) -> Placement {
        Placement::bin(x, y, BinSpec { cells: [w, d], ..BinSpec::default() })
    }

    #[test]
    fn small_drawer_layout_covers_its_grid() {
        let p = Preset::get("250x305").unwrap();
        let [nx, ny] = p.plate.cells();
        validate(nx, ny, &p.placements).unwrap();
        let bom: Vec<_> =
            bill_of_bins(&p.placements).into_iter().map(|(b, n)| (b.cells, b.units, b.dividers, n)).collect();
        assert_eq!(
            bom,
            [
                ([2, 2], 12, [1, 1], 2),
                ([1, 2], 12, [1, 1], 1),
                ([1, 5], 6, [1, 1], 1),
                ([2, 3], 6, [1, 1], 1),
                ([2, 2], 6, [1, 1], 1),
                ([2, 1], 6, [2, 1], 1),
                ([1, 1], 6, [1, 1], 8),
            ]
        );
    }

    #[test]
    fn overlap_and_overhang_are_rejected() {
        assert!(matches!(validate(2, 1, &[bin(0, 0, 1, 1), bin(0, 0, 2, 1)]), Err(LayoutError::NotExact { .. })));
        assert!(matches!(validate(2, 1, &[bin(1, 0, 2, 1)]), Err(LayoutError::OutOfGrid { .. })));
        assert!(matches!(validate(2, 1, &[bin(0, 0, 1, 1)]), Err(LayoutError::NotExact { .. })));
        validate(2, 1, &[bin(0, 0, 1, 1), bin(1, 0, 1, 1)]).unwrap();
    }

    #[test]
    fn repair_moves_drops_and_fills() {
        let h = Heights { back: 12, front: 6, max: 12 };
        // The second 2x2 has no room left, the 4x1 is wider than the grid; three cells get filled.
        let input = vec![bin(0, 0, 2, 2), bin(1, 1, 2, 2), bin(0, 0, 4, 1), bin(2, 0, 1, 1)];
        let (out, repairs) = repair(3, 3, input, h);
        validate(3, 3, &out).unwrap();
        assert_eq!(
            repairs,
            [
                Repair::Dropped { index: 1, cells: [2, 2] },
                Repair::Dropped { index: 2, cells: [4, 1] },
                Repair::Filled { count: 3 },
            ]
        );
        let (out, repairs) = repair(2, 1, vec![bin(0, 0, 1, 1), bin(0, 0, 1, 1)], h);
        assert_eq!(repairs, [Repair::Moved { index: 1, from: [0, 0], to: [1, 0] }]);
        assert_eq!((out[1].x, out[1].y), (1, 0));
    }
}
