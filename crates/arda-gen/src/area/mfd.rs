//! Deterministic Freeman p=1.1 fractional hillslope contribution, used only by erosion.
//! Final saved channels retain the single-receiver topology.
// The legacy square domain stays <=512^2 cells; the checked shared rectangle
// has at most u32::MAX cells. Total area <2^64 Q32; area*weight and the square
// root radicand <2^96. The temporary routing range is i32 terrain plus at most
// u32::MAX millimetres: slope score <2^44, normalization numerator <2^76.
// Fixed-width casts below are safe under those checked domain bounds.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
use super::fill::Filled;
use crate::hydrology::HydrologyError;
use arda_core::CellCoord;
pub(crate) const ONE: u64 = 1_u64 << 32;
const TABLE: &[u8] = include_bytes!("mfd-p11-q32.bin");
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
pub(crate) fn weight(x: u64) -> u64 {
    if x >= ONE {
        return ONE;
    }
    let k = (x >> 18) as usize;
    let r = x & ((1 << 18) - 1);
    let at = |i: usize| {
        u64::from_le_bytes([
            TABLE[i * 8],
            TABLE[i * 8 + 1],
            TABLE[i * 8 + 2],
            TABLE[i * 8 + 3],
            TABLE[i * 8 + 4],
            TABLE[i * 8 + 5],
            TABLE[i * 8 + 6],
            TABLE[i * 8 + 7],
        ])
    };
    let a = at(k);
    let b = at(k + 1);
    a + (((b - a) * r) >> 18)
}
pub(crate) fn isqrt(x: u128) -> u128 {
    if x < 2 {
        return x;
    }
    let mut a = 1_u128 << (128 - x.leading_zeros()).div_ceil(2);
    loop {
        let b = (a + x / a) / 2;
        if b >= a {
            return a;
        }
        a = b;
    }
}
pub(crate) fn incision(area_q32: u64, slope_permille: i64, time_den: i64) -> i64 {
    assert!(time_den > 0);
    let root = isqrt(u128::from(area_q32) << 32);
    ((root * slope_permille.max(0) as u128) / (u128::from(ONE) * time_den as u128)) as i64
}
pub(crate) fn accumulate_grid(h: &[i32], n: usize) -> Vec<u64> {
    assert!((2..=512).contains(&n));
    assert_eq!(h.len(), n * n);
    let mut order: Vec<_> = (0..h.len()).collect();
    order.sort_unstable_by(|&a, &b| h[b].cmp(&h[a]).then(a.cmp(&b)));
    let mut area = vec![ONE; h.len()];
    let mut terminal_total = 0_u128;
    for i in order {
        let (x, y) = ((i % n) as i32, (i / n) as i32);
        let mut next = [0_usize; 8];
        let mut scores = [0_u64; 8];
        let mut len = 0;
        for (dx, dy) in D {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= n as i32 || ny >= n as i32 {
                continue;
            }
            let j = ny as usize * n + nx as usize;
            let drop = i64::from(h[i]) - i64::from(h[j]);
            if drop <= 0 {
                continue;
            }
            next[len] = j;
            scores[len] = drop as u64 * if dx != 0 && dy != 0 { 1000 } else { 1414 };
            len += 1;
        }
        if len == 0 {
            terminal_total += u128::from(area[i]);
            continue;
        }
        let max = scores[..len].iter().copied().max().unwrap_or(0);
        let mut weights = [0_u64; 8];
        let mut sum = 0_u64;
        for k in 0..len {
            weights[k] = weight(((u128::from(scores[k]) << 32) / u128::from(max)) as u64);
            sum += weights[k];
        }
        assert!(sum > 0);
        let mut remaining = area[i];
        for k in 0..len {
            let share = if k + 1 == len {
                remaining
            } else {
                (u128::from(area[i]) * u128::from(weights[k]) / u128::from(sum)) as u64
            };
            remaining -= share;
            area[next[k]] += share;
        }
        assert_eq!(remaining, 0);
    }
    assert_eq!(
        terminal_total,
        h.len() as u128 * u128::from(ONE),
        "fractional catchment conservation"
    );
    area
}
pub(crate) fn accumulate(filled: &Filled) -> Vec<u64> {
    let h: Vec<_> = (0..512 * 512)
        .filter_map(|i| CellCoord::new((i % 512) as u16, (i / 512) as u16).map(|c| filled.get(c)))
        .collect();
    accumulate_grid(&h, 512)
}

/// Shared-domain variant (`logic/02`, shared terrain correction). The routing
/// flood supplies every row-major index exactly once in ascending level order.
/// Its reverse is already a topological traversal, avoiding a whole-world sort.
/// No allocation occurs here. At most u32::MAX cells keeps total Q32 area in u64;
/// the routing level interval is i32 terrain plus at most u32::MAX millimetres.
pub(crate) fn accumulate_rectangular(
    h: &[i64],
    width: usize,
    height: usize,
    order: &[u32],
    area: &mut [u64],
) -> Result<(), HydrologyError> {
    let count = width
        .checked_mul(height)
        .filter(|&count| width >= 2 && height >= 2 && count <= u32::MAX as usize)
        .ok_or(HydrologyError::TerrainPreparation("invalid MFD rectangle"))?;
    if h.len() != count || order.len() != count || area.len() != count {
        return Err(HydrologyError::TerrainPreparation("MFD buffer length"));
    }
    if h.iter().any(|&level| {
        level < i64::from(i32::MIN) || level > i64::from(i32::MAX) + i64::from(u32::MAX)
    }) {
        return Err(HydrologyError::TerrainPreparation(
            "MFD routing level range",
        ));
    }
    area.fill(ONE);
    let mut terminal_total = 0_u128;
    for &raw in order.iter().rev() {
        let i = raw as usize;
        if i >= count {
            return Err(HydrologyError::TerrainPreparation("MFD routing index"));
        }
        let (x, y) = ((i % width) as i64, (i / width) as i64);
        let mut next = [0_usize; 8];
        let mut scores = [0_u64; 8];
        let mut len = 0;
        for (dx, dy) in D {
            let (nx, ny) = (x + i64::from(dx), y + i64::from(dy));
            if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                continue;
            }
            let j = ny as usize * width + nx as usize;
            let drop = h[i] - h[j];
            if drop > 0 {
                next[len] = j;
                scores[len] = drop as u64 * if dx != 0 && dy != 0 { 1000 } else { 1414 };
                len += 1;
            }
        }
        if len == 0 {
            terminal_total += u128::from(area[i]);
            continue;
        }
        let max = scores[..len].iter().copied().max().unwrap_or(0);
        let mut weights = [0_u64; 8];
        let mut sum = 0_u64;
        for k in 0..len {
            weights[k] = weight(((u128::from(scores[k]) << 32) / u128::from(max)) as u64);
            sum += weights[k];
        }
        let mut remaining = area[i];
        for k in 0..len {
            let share = if k + 1 == len {
                remaining
            } else {
                (u128::from(area[i]) * u128::from(weights[k]) / u128::from(sum)) as u64
            };
            remaining -= share;
            area[next[k]] = area[next[k]]
                .checked_add(share)
                .ok_or(HydrologyError::Overflow("shared fractional catchment"))?;
        }
    }
    if terminal_total != count as u128 * u128::from(ONE) {
        return Err(HydrologyError::InconsistentTopology(
            "shared fractional catchment conservation",
        ));
    }
    Ok(())
}
