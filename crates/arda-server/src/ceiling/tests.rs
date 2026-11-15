//! The ceiling holds (logic/16 §api-memory): every fitted configuration's
//! admitted worst case, with what the world and library hold, stays under
//! the ceiling; a ceiling below the floors is refused, naming the least.

use super::*;
use crate::admission::Resident;
use crate::ServerConfig;

const MIB: u64 = 1 << 20;

/// The MICRO-42 kit world with the placeholder library.
const MICRO: Resident = Resident {
    library: 89 * MIB,
    society: 20 * MIB,
    shore: 4 * MIB,
    plans: 73 * MIB,
};
/// `out/site-renders/full42` (500 × 1,000 km) with the placeholder library.
const FULL: Resident = Resident {
    library: 89 * MIB,
    society: 773 * MIB,
    shore: 85 * MIB,
    plans: 462 * MIB,
};

fn config(ceiling: Option<u64>) -> ServerConfig {
    let mut c = ServerConfig::new("world".into());
    c.memory_ceiling = ceiling;
    c
}

/// DiceMaster's profile: prefetch on, baking off.
fn dicemaster(ceiling: Option<u64>) -> ServerConfig {
    let mut c = config(ceiling);
    c.bake_workers = 0;
    c
}

fn profiles(ceiling: u64) -> Vec<ServerConfig> {
    let mut out = vec![config(Some(ceiling)), dicemaster(Some(ceiling))];
    let mut quiet = dicemaster(Some(ceiling));
    quiet.prefetch_workers = 0;
    out.push(quiet);
    let mut pinned = config(Some(ceiling));
    pinned.serve.max_connections = 8;
    pinned.query.area_bytes = 200 << 20;
    pinned.pinned = Pinned {
        area_cache: true,
        max_connections: true,
        ..Pinned::default()
    };
    out.push(pinned);
    out
}

#[test]
fn without_a_ceiling_nothing_changes() {
    let c = config(None);
    assert_eq!(format!("{:?}", fit(&c, &FULL).unwrap()), format!("{c:?}"));
    assert_eq!(c.ceiling(), crate::MEMORY_CEILING_BYTES);
    assert_eq!(c.admitted_bytes() >> 20, 14_422);
}

#[test]
fn every_fitted_worst_case_stays_under_its_ceiling() {
    for resident in [MICRO, FULL] {
        for probe in profiles(16 << 30) {
            let least = smallest(&probe, &resident);
            let mut ceiling = least;
            while ceiling <= 16 << 30 {
                let mut c = probe.clone();
                c.memory_ceiling = Some(ceiling);
                let f = fit(&c, &resident).unwrap_or_else(|e| panic!("{ceiling}: {e}"));
                let need = f.admitted_bytes() + resident.total();
                assert!(need <= ceiling, "{need} > {ceiling}");
                f.admit().unwrap();
                f.admit_resident(&resident).unwrap();
                assert!(f.tactical.max_render_px >= ONE_CELL_64_PX);
                ceiling += 97 << 20;
            }
        }
    }
}

#[test]
fn a_ceiling_below_the_floors_is_refused_naming_the_least() {
    let c = dicemaster(Some(1 << 30));
    let least = smallest(&c, &FULL);
    let err = fit(&c, &FULL).unwrap_err().to_string();
    assert!(
        err.contains(&format!("{} MiB", least.div_ceil(MIB))),
        "{err}"
    );
    let mut at = c.clone();
    at.memory_ceiling = Some(least);
    assert!(fit(&at, &FULL).is_ok());
    at.memory_ceiling = Some(least - MIB);
    assert!(fit(&at, &FULL).is_err());
}

#[test]
fn budgets_grow_with_the_ceiling_up_to_the_defaults() {
    let mut last = fit(
        &dicemaster(Some(smallest(&dicemaster(None), &MICRO))),
        &MICRO,
    )
    .unwrap();
    for gib in 5..=16 {
        let f = fit(&dicemaster(Some(gib << 30)), &MICRO).unwrap();
        assert!(f.query.area_bytes >= last.query.area_bytes);
        assert!(f.tactical.render_cache_bytes >= last.tactical.render_cache_bytes);
        assert!(f.tactical.max_render_px >= last.tactical.max_render_px);
        assert!(f.pipeline.plan_cache_bytes >= last.pipeline.plan_cache_bytes);
        last = f;
    }
    let roomy = fit(&dicemaster(Some(64 << 30)), &MICRO).unwrap();
    let mut defaults = dicemaster(Some(64 << 30));
    defaults.memory_ceiling = roomy.memory_ceiling;
    assert_eq!(
        format!("{roomy:?}"),
        format!("{defaults:?}"),
        "a roomy ceiling keeps every default"
    );
}

#[test]
fn pinned_flags_keep_their_values() {
    let mut c = dicemaster(Some(8 << 30));
    c.query.fine_bytes = 300 << 20;
    c.serve.max_connections = 12;
    c.pinned = Pinned {
        fine_cache: true,
        max_connections: true,
        ..Pinned::default()
    };
    let f = fit(&c, &FULL).unwrap();
    assert_eq!(f.query.fine_bytes, 300 << 20);
    assert_eq!(f.serve.max_connections, 12);
}

/// The figures `crates/arda-server/API.md` § Memory ceiling quotes.
#[test]
fn the_documented_least_ceilings_hold() {
    let mib = |c: &ServerConfig, r: &Resident| smallest(c, r).div_ceil(MIB);
    assert_eq!(mib(&dicemaster(None), &FULL), 5_505);
    assert_eq!(mib(&dicemaster(None), &MICRO), 4_282);
}
