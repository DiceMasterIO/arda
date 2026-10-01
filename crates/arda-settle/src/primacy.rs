//! Primate capitals: rank-size within each realm (`logic/08` §settle-tiers,
//! §realm-seats; goal 35 "one or two cities per realm").
//!
//! A realm keeps the townspeople its towns were placed with, `U`. Within
//! the realm the towns follow the rank-size rule with the seat first: the
//! town of rank `r` holds `h / r` with `h = U / H_n`. A capital draws
//! court, garrison and trade beyond what the rule gives it (Jefferson's
//! primate city), so it holds at least a city's population, [`CITY_MIN`];
//! the court's trade lifts the realm's other towns a little too
//! ([`court_head`]). The people they draw come from the realm's villages
//! and hamlets, which shrink in proportion but never below their tier's
//! floor. No town falls below [`TOWN_MIN`]. Because each realm's list is a rank-size list, the union
//! over the land still fits the rule (logic/08 Invariants 4).
//!
//! A realm too small to feed a city (fewer than half of
//! [`PEOPLE_PER_REALM`](crate::seats::PEOPLE_PER_REALM) people) keeps its
//! towns as placed.

use crate::model::{Settlement, Tier, CITY_MIN, TOWN_MIN};
use crate::seats::PEOPLE_PER_REALM;

/// Harmonic number H(n) in millionths.
fn harmonic_micro(n: u64) -> u64 {
    (1..=n).map(|k| 1_000_000 / k).sum()
}

/// The rank-size head of a realm's towns once its capital has grown into
/// a city: the court's trade lifts the other towns by the fourth root of
/// the capital's growth (an elasticity of ¼, assumed, tunable), so the
/// realm's list keeps its shape below a primate capital and the union over
/// the land keeps a rank-size slope near −1.
fn court_head(head: u64) -> u64 {
    let cap = u64::from(CITY_MIN);
    if head == 0 || head >= cap {
        return head;
    }
    let h = u128::from(head);
    let v = h * h * h * u128::from(cap);
    u64::try_from(isqrt128(isqrt128(v)))
        .unwrap_or(head)
        .max(head)
}

fn isqrt128(v: u128) -> u128 {
    if v < 2 {
        return v;
    }
    let mut x = v;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    x
}

/// Population floor and ceiling of a rural tier.
const fn bounds(t: Tier) -> (u32, u32) {
    match t {
        Tier::Hamlet => (12, 99),
        _ => (100, 999),
    }
}

/// Rescales the rural `members` so they hold `want` people, each within its
/// tier's bounds (two proportional passes; a remainder of a few people
/// may stay when every member sits on a bound).
fn rescale_rural(settlements: &mut [Settlement], members: &[usize], want: u64) {
    for _ in 0..2 {
        let have: u64 = members
            .iter()
            .map(|&k| u64::from(settlements[k].population))
            .sum();
        if have == want || have == 0 {
            return;
        }
        // Only members off the bound in the direction of travel can move.
        let free: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&k| {
                let s = &settlements[k];
                let (lo, hi) = bounds(s.tier);
                if want < have {
                    s.population > lo
                } else {
                    s.population < hi
                }
            })
            .collect();
        let pool: u64 = free
            .iter()
            .map(|&k| u64::from(settlements[k].population))
            .sum();
        if pool == 0 {
            return;
        }
        let target = if want < have {
            pool.saturating_sub(have - want)
        } else {
            pool + (want - have)
        };
        for &k in &free {
            let s = &mut settlements[k];
            let (lo, hi) = bounds(s.tier);
            let v = u64::from(s.population) * target / pool;
            s.population = u32::try_from(v).unwrap_or(hi).clamp(lo, hi);
        }
    }
}

/// Resizes towns and rural settlements realm by realm. `realm_of[k]` is the
/// seat slot settlement `k` swears to and `seats[slot]` the seat's index.
pub fn apply(settlements: &mut [Settlement], seats: &[usize], realm_of: &[usize]) {
    for (slot, &seat) in seats.iter().enumerate() {
        if !settlements[seat].tier.is_urban() {
            continue;
        }
        let members: Vec<usize> = (0..settlements.len())
            .filter(|&k| realm_of[k] == slot)
            .collect();
        let people: u64 = members
            .iter()
            .map(|&k| u64::from(settlements[k].population))
            .sum();
        if people < PEOPLE_PER_REALM / 2 {
            continue;
        }
        let mut towns: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&k| k != seat && settlements[k].tier.is_urban())
            .collect();
        towns.sort_by(|&a, &b| {
            settlements[b]
                .population
                .cmp(&settlements[a].population)
                .then(settlements[a].id.cmp(&settlements[b].id))
        });
        towns.insert(0, seat);
        let urban: u64 = towns
            .iter()
            .map(|&k| u64::from(settlements[k].population))
            .sum();
        let n = u64::try_from(towns.len()).unwrap_or(1);
        let head = court_head(urban * 1_000_000 / harmonic_micro(n).max(1));
        let mut placed = 0_u64;
        for (r, &k) in towns.iter().enumerate() {
            let rank = u64::try_from(r + 1).unwrap_or(1);
            let floor = if r == 0 { CITY_MIN } else { TOWN_MIN };
            let pop = u32::try_from(head / rank).unwrap_or(u32::MAX).max(floor);
            settlements[k].population = pop;
            settlements[k].tier = Tier::of_town(pop);
            placed += u64::from(pop);
        }
        let rural: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&k| !settlements[k].tier.is_urban())
            .collect();
        rescale_rural(settlements, &rural, people.saturating_sub(placed));
    }
    rerank(settlements);
}

/// Ranks towns and cities by population (1 = largest, ties by id); 0 for
/// villages and hamlets.
pub fn rerank(settlements: &mut [Settlement]) {
    let mut urban: Vec<usize> = (0..settlements.len())
        .filter(|&k| settlements[k].tier.is_urban())
        .collect();
    urban.sort_by(|&a, &b| {
        settlements[b]
            .population
            .cmp(&settlements[a].population)
            .then(settlements[a].id.cmp(&settlements[b].id))
    });
    for s in settlements.iter_mut() {
        s.rank = 0;
    }
    for (r, &k) in urban.iter().enumerate() {
        settlements[k].rank = u32::try_from(r + 1).unwrap_or(u32::MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn realm() -> Vec<Settlement> {
        let mut s = vec![
            Settlement::bare(1, 0, 0, Tier::Town, 4000),
            Settlement::bare(2, 5, 0, Tier::Town, 3000),
            Settlement::bare(3, 9, 0, Tier::Town, 1200),
        ];
        for k in 0..60 {
            s.push(Settlement::bare(4 + k, 1, 1, Tier::Village, 400));
        }
        for k in 0..60 {
            s.push(Settlement::bare(64 + k, 2, 2, Tier::Hamlet, 50));
        }
        s
    }

    #[test]
    fn the_capital_becomes_a_primate_city_and_towns_follow_rank_size() {
        let mut s = realm();
        let before: u64 = s.iter().map(|x| u64::from(x.population)).sum();
        // The seat is the second town: it still ends up first.
        let all = vec![0; s.len()];
        apply(&mut s, &[1], &all);
        assert_eq!(s[1].tier, Tier::City);
        assert!(s[1].population >= CITY_MIN);
        assert_eq!(s[1].rank, 1);
        assert!(s[0].population > s[2].population && s[2].population >= TOWN_MIN);
        assert_eq!(s.iter().filter(|x| x.tier == Tier::City).count(), 1);
        for x in &s[3..] {
            let (lo, hi) = bounds(x.tier);
            assert!((lo..=hi).contains(&x.population), "{x:?}");
        }
        let after: u64 = s.iter().map(|x| u64::from(x.population)).sum();
        assert!(after.abs_diff(before) * 100 <= before, "{before} → {after}");
    }

    #[test]
    fn a_small_realm_keeps_its_towns() {
        let mut s = vec![
            Settlement::bare(1, 0, 0, Tier::Town, 2000),
            Settlement::bare(2, 0, 0, Tier::Village, 300),
        ];
        apply(&mut s, &[0], &[0, 0]);
        assert_eq!(s[0].population, 2000);
        assert_eq!(s[0].rank, 1);
    }
}
