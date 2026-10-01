//! Legality checks of finished grids (for review and tests).

use crate::rules::Dir;
use crate::solve::Problem;

/// Illegal places in a finished grid: `(cell, side)` where a tile's edge
/// kind is not allowed or its neighbour across that side is not.
#[must_use]
pub fn violations(p: &Problem<'_>, tiles: &[u16]) -> Vec<(usize, Dir)> {
    let mut bad = Vec::new();
    for (i, &t) in tiles.iter().enumerate() {
        let t = usize::from(t);
        if !p.domains.get(i).is_some_and(|d| d.contains(t)) {
            bad.push((i, Dir::N));
            continue;
        }
        for d in Dir::ALL {
            let k = p.kind(i, d);
            let ok = match p.neighbour(i, d) {
                Some(j) => {
                    let b = usize::from(tiles.get(j).copied().unwrap_or(0));
                    p.rules.legal(t, d, b, k)
                }
                None => p.rules.edge_ok(t, d, k),
            };
            if !ok {
                bad.push((i, d));
            }
        }
    }
    bad
}
