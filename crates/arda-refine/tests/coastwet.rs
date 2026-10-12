//! The coast and wetland statistics tool on the synthetic world: exact
//! shares, determinism, and agreement of the gathered coast facts with the
//! direct reads.
#![allow(clippy::unwrap_used, missing_docs)]

use arda_core::ShoreClass;
use arda_refine::coastwet::{coast_of, measure};
use arda_refine::context::Ctx;
use arda_refine::synthetic::{self, SEA_ROW, W};
use arda_refine::{CellKey, Source};

#[test]
fn shares_on_the_synthetic_world_are_exact() {
    let src = synthetic::world(42);
    let r = measure(&src, 1).unwrap();
    let all = &r.groups["all"];
    // 16 x 13 land rows, minus the four lake cells.
    assert_eq!(all.land_cells, 16 * 13 - 4);
    // Two marsh cells and the sixteen river-row cells (wetness 200 of 255,
    // above the marsh band).
    assert!(
        (all.marshy_pct - 1800.0 / 204.0).abs() < 1e-3,
        "{}",
        all.marshy_pct
    );
    assert!(!r.has_shore_layer);
    // The row above the sea is the only coast: sixteen cells, no classes.
    assert_eq!(r.strip["unclassified"].cells, u64::from(W));
    assert_eq!(r.strip.len(), 1);
}

#[test]
fn the_report_is_deterministic() {
    let src = synthetic::world(7);
    let a = serde_json::to_string(&measure(&src, 2).unwrap()).unwrap();
    let b = serde_json::to_string(&measure(&src, 2).unwrap()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn gathered_coast_facts_equal_direct_reads() {
    let mut src = synthetic::world(42);
    src.set_shore(CellKey::new(5, SEA_ROW - 1), ShoreClass::SandBeach);
    for k in [
        CellKey::new(5, SEA_ROW - 1),
        CellKey::new(5, SEA_ROW - 3),
        CellKey::new(0, SEA_ROW - 1),
        CellKey::new(8, 4),
        CellKey::new(5, SEA_ROW),
    ] {
        let ctx = Ctx::gather(&src, k).unwrap();
        for dy in -2..=2 {
            for dx in -2..=2 {
                let at = k.offset(dx, dy);
                if src.clamp(at) != at {
                    continue;
                }
                assert_eq!(ctx.coast_at(at), coast_of(&src, at).unwrap(), "{at:?}");
            }
        }
    }
    let c = coast_of(&src, CellKey::new(5, SEA_ROW - 3)).unwrap();
    assert_eq!((c.dist, c.shore), (3, ShoreClass::SandBeach));
}
