//! Interior variety (goal 64: no visible repetition). A building's interior
//! is a pure function of its plan, its own id and a small *salt*. Salts are
//! chosen once per plan, greedily in building order, so that no building's
//! interior equals that of an adjacent building drawn before it: adjacent
//! houses never share a layout, and the choice still depends only on the
//! plan, never on the window being cut (goal 46).
//!
//! Interiors are compared by a fingerprint of their canonical frame (the
//! street front to the north): width, depth, walls, floors and props.

use super::{canon, Canon};
use crate::function::BuildingFunction as F;
use crate::plan::{Building, TownPlan};
use crate::rng::hash_str;
use arda_tactical::catalog::WallRole;
use rayon::prelude::*;
use std::collections::BTreeMap;

/// Salts tried before a building keeps its first choice.
pub const SALTS: u8 = 8;

/// Whether a function is a dwelling (goal 64's houses).
#[must_use]
pub fn is_home(f: F) -> bool {
    matches!(f, F::House | F::Cottage | F::Farmhouse | F::Manor)
}

fn role_key(r: WallRole) -> &'static str {
    match r {
        WallRole::Run => "r",
        WallRole::Door => "d",
        WallRole::Gate => "g",
        WallRole::Window => "w",
        _ => "o",
    }
}

/// Fingerprint of a canonical interior.
#[must_use]
pub fn fingerprint_canon(c: &Canon) -> u64 {
    let mut h = hash_str(0x1A7E, &format!("{}x{}", c.w, c.d));
    for (&(x, y, hz), &(role, kit)) in &c.walls {
        h = hash_str(h, &format!("w{x},{y},{hz},{},{kit}", role_key(role)));
    }
    for f in &c.floor {
        h = hash_str(h, f);
    }
    let mut props: Vec<String> = c
        .props
        .iter()
        .map(|p| {
            let want = match &p.want {
                super::Want::Id(id) => (*id).to_owned(),
                super::Want::Query(t) => t.join("+"),
            };
            format!("{want}@{},{},{},{}r{}", p.x0, p.y0, p.x1, p.y1, p.rot)
        })
        .collect();
    props.sort();
    for p in &props {
        h = hash_str(h, p);
    }
    h
}

/// Building positions adjacent to each building: footprints touching or
/// one square apart (diagonals included).
#[must_use]
pub fn neighbours(plan: &TownPlan) -> Vec<Vec<usize>> {
    let bs = &plan.buildings;
    let mut order: Vec<usize> = (0..bs.len()).collect();
    order.sort_by_key(|&i| (bs[i].rect.x0, i));
    let mut out = vec![Vec::new(); bs.len()];
    for (k, &i) in order.iter().enumerate() {
        let near = bs[i].rect.grown(1);
        for &j in &order[k + 1..] {
            if bs[j].rect.x0 >= near.x1 {
                break;
            }
            if near.overlaps(&bs[j].rect) {
                out[i].push(j);
                out[j].push(i);
            }
        }
    }
    for n in &mut out {
        n.sort_unstable();
    }
    out
}

/// The salt of every building, by position: the first salt whose interior
/// differs from those of every adjacent, earlier building.
#[must_use]
pub fn salts(plan: &TownPlan) -> Vec<u8> {
    let bs = &plan.buildings;
    // Sequential: this runs inside `OnceLock::get_or_init`, where a
    // parallel iterator could steal a job that waits on the same lock.
    let first: Vec<u64> = bs.iter().map(|b| fingerprint_with(plan, b, 0)).collect();
    let near = neighbours(plan);
    let mut chosen: Vec<(u8, u64)> = Vec::with_capacity(bs.len());
    for (i, b) in bs.iter().enumerate() {
        let taken: Vec<u64> = near[i]
            .iter()
            .filter(|&&j| j < i)
            .map(|&j| chosen[j].1)
            .collect();
        let mut pick = (0, first[i]);
        if taken.contains(&first[i]) {
            for s in 1..SALTS {
                let fp = fingerprint_with(plan, b, s);
                if !taken.contains(&fp) {
                    pick = (s, fp);
                    break;
                }
            }
        }
        chosen.push(pick);
    }
    chosen.into_iter().map(|c| c.0).collect()
}

/// The fingerprint of `b`'s interior drawn with `salt`.
#[must_use]
pub fn fingerprint_with(plan: &TownPlan, b: &Building, salt: u8) -> u64 {
    fingerprint_canon(&canon(plan, b, salt).0)
}

/// The fingerprint of `b`'s interior as built.
#[must_use]
pub fn fingerprint(plan: &TownPlan, b: &Building) -> u64 {
    fingerprint_with(plan, b, plan.interior_salt(b))
}

/// How varied a plan's home interiors are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diversity {
    /// Homes in the plan.
    pub homes: usize,
    /// Homes whose interior equals another home's.
    pub repeated: usize,
    /// Distinct home interiors.
    pub distinct: usize,
    /// Adjacent pairs of homes with equal interiors.
    pub adjacent_equal: usize,
}

impl Diversity {
    /// Fraction of homes whose interior is shared with another home.
    #[must_use]
    pub fn repeated_fraction(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)] // counts far below 2^52
        let f = self.repeated as f64 / self.homes.max(1) as f64;
        f
    }
}

/// Measures the variety of `plan`'s home interiors under the rule
/// programmes.
#[must_use]
pub fn diversity(plan: &TownPlan) -> Diversity {
    diversity_by(plan, |b| fingerprint(plan, b))
}

/// Measures the variety of `plan`'s home interiors as `fp` fingerprints
/// them (the WFC strategy passes its own drawing).
#[must_use]
pub fn diversity_by(plan: &TownPlan, fp: impl Fn(&Building) -> u64 + Sync) -> Diversity {
    let bs = &plan.buildings;
    if let Some(b) = bs.first() {
        // Draw the salts before any parallel reader waits on them.
        let _ = plan.interior_salt(b);
    }
    let fps: Vec<Option<u64>> = bs
        .par_iter()
        .map(|b| is_home(b.function).then(|| fp(b)))
        .collect();
    let mut count: BTreeMap<u64, usize> = BTreeMap::new();
    for fp in fps.iter().flatten() {
        *count.entry(*fp).or_default() += 1;
    }
    let near = neighbours(plan);
    let adjacent_equal = near
        .iter()
        .enumerate()
        .flat_map(|(i, n)| n.iter().filter(move |&&j| j > i).map(move |&j| (i, j)))
        .filter(|&(i, j)| fps[i].is_some() && fps[i] == fps[j])
        .count();
    Diversity {
        homes: fps.iter().flatten().count(),
        repeated: count.values().filter(|&&n| n > 1).sum(),
        distinct: count.len(),
        adjacent_equal,
    }
}
