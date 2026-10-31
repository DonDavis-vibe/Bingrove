//! Arranging parts on printer beds: shelves from the front, tallest-footprint first, parts
//! turned by 90° when that makes a shelf lower.

use crate::model::BedSpec;

/// Space between parts on a bed.
pub const SPACING: f64 = 5.0;

/// Where a footprint ended up: bed index, quarter turns, minimum corner on the bed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    pub bed: usize,
    pub turned: bool,
    pub at: [f64; 2],
}

/// Places footprints `(w, d)` on as few beds as the shelf rule allows. Footprints that fit no
/// bed get their own bed, at the margin.
pub fn pack(sizes: &[[f64; 2]], bed: &BedSpec) -> Vec<Spot> {
    let (bw, bd) = (bed.x - 2.0 * bed.margin, bed.y - 2.0 * bed.margin);
    // Orientation per part: the shorter side as shelf depth when the longer one fits across.
    let oriented: Vec<([f64; 2], bool)> =
        sizes.iter().map(|&[w, d]| if d > w && d <= bw + 1e-9 { ([d, w], true) } else { ([w, d], false) }).collect();
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| {
        oriented[b].0[1]
            .total_cmp(&oriented[a].0[1])
            .then(oriented[b].0[0].total_cmp(&oriented[a].0[0]))
            .then(a.cmp(&b))
    });

    let mut spots = vec![Spot { bed: 0, turned: false, at: [0.0; 2] }; sizes.len()];
    // Per bed: shelves as (y, depth, x used).
    let mut beds: Vec<Vec<(f64, f64, f64)>> = Vec::new();
    for i in order {
        let ([w, d], turned) = oriented[i];
        let mut placed = false;
        'beds: for (b, shelves) in beds.iter_mut().enumerate() {
            for shelf in shelves.iter_mut() {
                if d <= shelf.1 + 1e-9 && shelf.2 + w <= bw + 1e-9 {
                    spots[i] = Spot { bed: b, turned, at: [bed.margin + shelf.2, bed.margin + shelf.0] };
                    shelf.2 += w + SPACING;
                    placed = true;
                    break 'beds;
                }
            }
            let y = shelves.last().map_or(0.0, |s| s.0 + s.1 + SPACING);
            if y + d <= bd + 1e-9 && w <= bw + 1e-9 {
                spots[i] = Spot { bed: b, turned, at: [bed.margin, bed.margin + y] };
                shelves.push((y, d, w + SPACING));
                placed = true;
                break;
            }
        }
        if !placed {
            spots[i] = Spot { bed: beds.len(), turned, at: [bed.margin, bed.margin] };
            beds.push(vec![(0.0, d, w + SPACING)]);
        }
    }
    spots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_parts_share_a_bed() {
        let bed = BedSpec::default();
        let spots = pack(&[[41.5, 41.5]; 8], &bed);
        assert!(spots.iter().all(|s| s.bed == 0));
        // Five fit across 240 mm (5 x 41.5 + 4 x 5), the rest go on the next shelf.
        assert_eq!(spots.iter().filter(|s| s.at[1] == 5.0).count(), 5);
    }

    #[test]
    fn long_parts_lie_across_and_big_ones_get_their_own_bed() {
        let bed = BedSpec::default();
        let spots = pack(&[[41.5, 209.5], [212.0, 173.0], [212.0, 131.0]], &bed);
        assert!(spots[0].turned);
        assert_ne!(spots[1].bed, spots[2].bed);
        let beds = spots.iter().map(|s| s.bed).max().unwrap() + 1;
        assert_eq!(beds, 2);
    }
}
