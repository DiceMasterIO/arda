//! Exact physical depression surface. Never stores routing epsilon in terrain.
use std::{cmp::Reverse, collections::BinaryHeap};
const D: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];
// n<=512 makes all coordinate casts bounded; negative neighbors are rejected before unsigned casts.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
pub(crate) fn surface(heights: &[i32], n: usize) -> Vec<i32> {
    assert!(n > 1 && n <= 512);
    assert_eq!(heights.len(), n * n);
    let mut result = heights.to_vec();
    let mut seen = vec![false; heights.len()];
    let mut queue = BinaryHeap::new();
    for y in 0..n {
        for x in 0..n {
            if x == 0 || y == 0 || x == n - 1 || y == n - 1 {
                let i = y * n + x;
                seen[i] = true;
                queue.push(Reverse((heights[i], y, x)));
            }
        }
    }
    while let Some(Reverse((level, y, x))) = queue.pop() {
        for (dx, dy) in D {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= n as i32 || ny >= n as i32 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            let i = ny * n + nx;
            if seen[i] {
                continue;
            }
            seen[i] = true;
            let physical = heights[i].max(level);
            result[i] = physical;
            queue.push(Reverse((physical, ny, nx)));
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flat_is_not_a_physical_basin() {
        let h = vec![100; 49];
        assert_eq!(surface(&h, 7), h);
    }
    #[test]
    fn physical_bowl_preserves_exact_spill() {
        let mut h = vec![100; 49];
        h[24] = 30;
        let s = surface(&h, 7);
        assert_eq!(s[24], 100);
        assert_eq!(s.iter().zip(&h).filter(|(a, b)| a > b).count(), 1);
    }
    #[test]
    fn below_sea_closed_bowl_is_still_physical() {
        let mut h = vec![100; 49];
        h[24] = -300;
        assert_eq!(surface(&h, 7)[24], 100);
    }
    #[test]
    fn minimum_integer_is_a_valid_height_not_a_visit_marker() {
        let h = vec![i32::MIN; 49];
        assert_eq!(surface(&h, 7), h);
    }
}
