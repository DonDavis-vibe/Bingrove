//! "Ten compartments in this drawer": which mixes of bin sizes make a wanted number of bins out
//! of the grid, and how a chosen mix tiles it. Every mix offered is known to fit.

use std::collections::BTreeMap;

use crate::model::{BinSpec, Placement};
use crate::solver::{Heights, back_row};

/// Bin footprints (`w >= d`, either way round when placed) with their counts.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Mix {
    pub sizes: Vec<([u32; 2], u32)>,
}

impl Mix {
    /// From footprints as they lie, largest first.
    pub fn of(footprints: impl IntoIterator<Item = [u32; 2]>) -> Mix {
        let mut counts: BTreeMap<[u32; 2], u32> = BTreeMap::new();
        for c in footprints {
            *counts.entry(normal(c)).or_default() += 1;
        }
        let mut sizes: Vec<_> = counts.into_iter().collect();
        sizes.sort_by_key(|(c, _)| std::cmp::Reverse((c[0] * c[1], c[0])));
        Mix { sizes }
    }

    pub fn count(&self) -> u32 {
        self.sizes.iter().map(|(_, n)| n).sum()
    }

    pub fn area(&self) -> u32 {
        self.sizes.iter().map(|([w, d], n)| w * d * n).sum()
    }

    /// The count of one footprint.
    pub fn get(&self, size: [u32; 2]) -> u32 {
        let size = normal(size);
        self.sizes.iter().find(|(c, _)| *c == size).map_or(0, |(_, n)| *n)
    }

    /// The same mix with `size` counted `n` times.
    pub fn with(&self, size: [u32; 2], n: u32) -> Mix {
        let size = normal(size);
        let mut sizes: Vec<_> = self.sizes.iter().filter(|(c, _)| *c != size).cloned().collect();
        if n > 0 {
            sizes.push((size, n));
        }
        sizes.sort_by_key(|(c, _)| std::cmp::Reverse((c[0] * c[1], c[0])));
        Mix { sizes }
    }

    /// "5 × 2×2 + 5 × 2×1"
    pub fn text(&self) -> String {
        self.sizes
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|([w, d], n)| format!("{n} × {w}×{d}"))
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

fn normal([w, d]: [u32; 2]) -> [u32; 2] {
    if w >= d { [w, d] } else { [d, w] }
}

/// Footprints worth offering: sides up to six cells and within the grid, `w >= d`.
pub fn footprints([nx, ny]: [u32; 2]) -> Vec<[u32; 2]> {
    let long = nx.max(ny).min(6);
    let short = nx.min(ny).min(6);
    let mut out = Vec::new();
    for w in 1..=long {
        for d in 1..=w.min(short) {
            out.push([w, d]);
        }
    }
    out
}

/// How far a mix is from "all the same, about the mean size, square": lower is better.
fn score(mix: &Mix, mean: f64) -> f64 {
    let k = mix.count() as f64;
    let (mut spread, mut aspect, mut tiny) = (0.0, 0.0, 0.0);
    for &([w, d], n) in &mix.sizes {
        let n = n as f64;
        spread += n * (((w * d) as f64 - mean) / mean).powi(2);
        aspect += n * (w as f64 / d as f64 - 1.0);
        if w * d == 1 && mean >= 2.0 {
            tiny += n;
        }
    }
    mix.sizes.len() as f64 + spread / k + 0.25 * aspect / k + 0.5 * tiny / k
}

/// Up to `max` mixes of one to three footprints that make `count` bins out of the grid, the most
/// even first. Every one of them tiles the grid.
pub fn suggest(cells: [u32; 2], count: u32, max: usize) -> Vec<Mix> {
    let area = cells[0] * cells[1];
    if count == 0 || count > area || max == 0 {
        return Vec::new();
    }
    let sizes = footprints(cells);
    let a = |s: [u32; 2]| s[0] * s[1];
    let mut candidates: Vec<Mix> = Vec::new();
    for (i, &s1) in sizes.iter().enumerate() {
        if a(s1) * count == area {
            candidates.push(Mix { sizes: vec![(s1, count)] });
        }
        for (j, &s2) in sizes.iter().enumerate().skip(i + 1) {
            for n1 in 1..count {
                let n2 = count - n1;
                if a(s1) * n1 + a(s2) * n2 == area {
                    candidates.push(Mix::of(
                        std::iter::repeat_n(s1, n1 as usize).chain(std::iter::repeat_n(s2, n2 as usize)),
                    ));
                }
            }
            for &s3 in sizes.iter().skip(j + 1) {
                for n1 in 1..count {
                    for n2 in 1..count - n1 {
                        let n3 = count - n1 - n2;
                        if a(s1) * n1 + a(s2) * n2 + a(s3) * n3 == area {
                            let all = std::iter::repeat_n(s1, n1 as usize)
                                .chain(std::iter::repeat_n(s2, n2 as usize))
                                .chain(std::iter::repeat_n(s3, n3 as usize));
                            candidates.push(Mix::of(all));
                        }
                    }
                }
            }
        }
    }
    let mean = area as f64 / count as f64;
    candidates.sort_by(|x, y| score(x, mean).total_cmp(&score(y, mean)));
    let mut out = Vec::new();
    for mix in candidates.into_iter().take(60) {
        if tile(cells, &mix).is_some() {
            out.push(mix);
            if out.len() == max {
                break;
            }
        }
    }
    out
}

/// The bins of a mix on the grid as `(x, y, cells)`, the back row first and large bins first;
/// `None` when they do not fit together (or the search would take too long).
pub fn tile(cells: [u32; 2], mix: &Mix) -> Option<Vec<(u32, u32, [u32; 2])>> {
    let [nx, ny] = cells;
    if mix.area() != nx * ny {
        return None;
    }
    let mut left: Vec<([u32; 2], u32)> = mix.sizes.iter().filter(|(_, n)| *n > 0).cloned().collect();
    left.sort_by_key(|(c, _)| std::cmp::Reverse(c[0] * c[1]));
    let mut taken = vec![false; (nx * ny) as usize];
    let mut out = Vec::new();
    let mut budget = 200_000u32;
    search(nx, ny, &mut taken, &mut left, &mut out, &mut budget).then(|| {
        out.sort_by_key(|&(x, y, _)| (y, x));
        out
    })
}

fn search(
    nx: u32,
    ny: u32,
    taken: &mut [bool],
    left: &mut [([u32; 2], u32)],
    out: &mut Vec<(u32, u32, [u32; 2])>,
    budget: &mut u32,
) -> bool {
    // The back-most, left-most free cell has to be a bin's back-left corner.
    let free = |taken: &[bool], x: u32, y: u32| !taken[(y * nx + x) as usize];
    let Some((x, y)) = (0..ny).rev().flat_map(|y| (0..nx).map(move |x| (x, y))).find(|&(x, y)| free(taken, x, y))
    else {
        return true;
    };
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    for i in 0..left.len() {
        if left[i].1 == 0 {
            continue;
        }
        let [w, d] = left[i].0;
        let ways: &[[u32; 2]] = if w == d { &[[w, d]] } else { &[[w, d], [d, w]] };
        for &c in ways {
            let [w, d] = c;
            if x + w > nx || y + 1 < d {
                continue;
            }
            let y0 = y + 1 - d;
            if !(y0..=y).all(|j| (x..x + w).all(|i| free(taken, i, j))) {
                continue;
            }
            for j in y0..=y {
                for i in x..x + w {
                    taken[(j * nx + i) as usize] = true;
                }
            }
            left[i].1 -= 1;
            out.push((x, y0, c));
            if search(nx, ny, taken, left, out, budget) {
                return true;
            }
            out.pop();
            left[i].1 += 1;
            for j in y0..=y {
                for i in x..x + w {
                    taken[(j * nx + i) as usize] = false;
                }
            }
        }
    }
    false
}

/// The mix laid out as placements: the back band's height for bins reaching behind the back
/// row, the front's for the rest.
pub fn place(cells: [u32; 2], mix: &Mix, h: Heights) -> Option<Vec<Placement>> {
    let split = back_row(cells[1]);
    let placed = tile(cells, mix)?;
    Some(
        placed
            .into_iter()
            .map(|(x, y, c)| {
                let units = if y + c[1] > split { h.back } else { h.front };
                Placement::bin(x, y, BinSpec { cells: c, units, ..BinSpec::default() })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::validate;

    fn heights() -> Heights {
        Heights { back: 12, front: 6, max: 12 }
    }

    #[test]
    fn ten_compartments_in_five_by_six() {
        let mixes = suggest([5, 6], 10, 5);
        assert!(!mixes.is_empty());
        for m in &mixes {
            assert_eq!((m.count(), m.area()), (10, 30), "{}", m.text());
            let p = place([5, 6], m, heights()).unwrap_or_else(|| panic!("{} does not tile", m.text()));
            validate(5, 6, &p).unwrap();
            assert_eq!(Mix::of(p.iter().map(|p| p.bin.cells)), *m);
        }
        // Ten 3 x 1 bins fill 5 x 6 standing up; the evenest mix comes first.
        assert_eq!(mixes[0], Mix { sizes: vec![([3, 1], 10)] });
        assert!(mixes.iter().any(|m| m.sizes.len() == 2), "a two-size mix is offered too");
        // Bins behind the back row are tall.
        let p = place([5, 6], &mixes[0], heights()).unwrap();
        assert!(p.iter().all(|b| b.bin.units == if b.y + b.bin.cells[1] > 4 { 12 } else { 6 }));
    }

    #[test]
    fn edge_cases() {
        assert_eq!(suggest([5, 7], 35, 3), vec![Mix { sizes: vec![([1, 1], 35)] }]);
        assert!(suggest([5, 7], 36, 3).is_empty());
        assert!(suggest([5, 7], 0, 3).is_empty());
        // Area right, but 3 x 3 bins cannot share a 3 x 6 grid with a 3 x 3 hole of 1 x 1s... they can;
        // a 2 x 2 in a 3-wide grid with 1 x 1s does, so ask for something that does not fit:
        // four 2 x 2 bins in 3 x 6 (area 16 != 18) is refused by area, two 3 x 3 plus nothing fits.
        assert!(tile([3, 6], &Mix { sizes: vec![([2, 2], 4)] }).is_none());
        assert!(tile([3, 6], &Mix { sizes: vec![([3, 3], 2)] }).is_some());
        // A 2 x 2 bin and 1 x 1 bins in a 3 x 3 grid fit (five 1 x 1s), four 2 x 1s and a 1 x 1 too.
        assert!(tile([3, 3], &Mix { sizes: vec![([2, 2], 1), ([1, 1], 5)] }).is_some());
        assert!(tile([3, 3], &Mix { sizes: vec![([2, 1], 4), ([1, 1], 1)] }).is_some());
    }

    #[test]
    fn mix_edits() {
        let m = Mix::of([[2, 2], [2, 2], [1, 2], [2, 1]]);
        assert_eq!(m.sizes, vec![([2, 2], 2), ([2, 1], 2)]);
        assert_eq!(m.get([1, 2]), 2);
        assert_eq!(m.with([1, 2], 0).sizes, vec![([2, 2], 2)]);
        assert_eq!(m.with([3, 1], 1).text(), "2 × 2×2 + 1 × 3×1 + 2 × 2×1");
    }
}
