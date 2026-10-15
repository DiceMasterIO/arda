//! Primate capitals (`logic/08` §settle-tiers, §realm-seats; goal 35 "one
//! or two cities per realm").
//!
//! Towns come out of placement sized by their hinterlands and sites
//! (`townsize.rs`). A capital draws court, garrison and trade beyond that
//! (Jefferson's primate city), so it holds at least a city's population,
//! [`CITY_MIN`]; the court's trade lifts the realm's other towns a little
//! too ([`court_head`]), keeping their order. The people they draw come
//! from the realm's villages and hamlets, which shrink in proportion but
//! never below their tier's floor. No town falls below [`TOWN_MIN`]. No
//! rank-size list is imposed, so the rank-size check over the land
//! (logic/08 Invariants 4) tests the outcome (review 3.3-12).
//!
//! A realm too small to feed a city (fewer than half of
//! [`PEOPLE_PER_REALM`](crate::seats::PEOPLE_PER_REALM) people) keeps its
//! towns as placed, except that its seat trades places with any larger
//! member.
//!
//! Realms here are the final partition (`realms::territory`), drawn before
//! this runs, so no town is sized for a realm it does not end up in
//! (review 3.3-8). [`cap`] then keeps at most [`MAX_CITIES`] cities a
//! realm and moves the surplus people within the realm (review 3.3-9).
//! Both run before land use, functions and roads, which therefore serve
//! the final populations.

use crate::model::{Settlement, Tier, CITY_MIN, TOWN_MIN};
use crate::seats::PEOPLE_PER_REALM;

/// At most this many cities per realm (goal 35).
pub const MAX_CITIES: usize = 2;

/// A town of `old` people in a realm whose capital grew from `was` to
/// `now`: the court's trade lifts it by the fourth root of the capital's
/// growth, `old · (now / was)^¼` (an elasticity of ¼, assumed, tunable).
fn court_head(old: u64, was: u64, now: u64) -> u64 {
    if was == 0 || now <= was {
        return old;
    }
    let o = u128::from(old);
    let v = o * o * o * o * u128::from(now) / u128::from(was);
    u64::try_from(isqrt128(isqrt128(v))).unwrap_or(old).max(old)
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

/// Grows each realm's seat into a primate capital. `realm_of[k]` is the
/// seat slot settlement `k` swears to and `seats[slot]` the seat's index.
///
/// The seat first trades places with any larger town of its realm (the
/// court draws the realm's first town); it then holds at least
/// [`CITY_MIN`], and when it grows by a factor `f` the realm's other towns
/// grow by `f^¼` ([`court_head`]). Their order and relative sizes, which
/// came from their hinterlands and sites (`townsize.rs`), are kept: no
/// rank-size list is imposed (review 3.3-12). The people come from the
/// realm's villages and hamlets.
pub fn apply(settlements: &mut [Settlement], seats: &[usize], realm_of: &[usize]) {
    for (slot, &seat) in seats.iter().enumerate() {
        let members: Vec<usize> = (0..settlements.len())
            .filter(|&k| realm_of[k] == slot)
            .collect();
        seat_first(settlements, seat, &members);
        if !settlements[seat].tier.is_urban() {
            continue;
        }
        let people: u64 = members
            .iter()
            .map(|&k| u64::from(settlements[k].population))
            .sum();
        if people < PEOPLE_PER_REALM / 2 {
            continue;
        }
        let was = u64::from(settlements[seat].population).max(1);
        let now = was.max(u64::from(CITY_MIN));
        let towns: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&k| k != seat && settlements[k].tier.is_urban())
            .collect();
        let mut urban = now;
        settlements[seat].population = u32::try_from(now).unwrap_or(u32::MAX);
        settlements[seat].tier = Tier::of_town(settlements[seat].population);
        for &k in &towns {
            let old = u64::from(settlements[k].population);
            // old · (now / was)^¼, kept below the seat.
            let grown = court_head(old, was, now)
                .min(now - 1)
                .max(u64::from(TOWN_MIN));
            settlements[k].population = u32::try_from(grown).unwrap_or(u32::MAX);
            settlements[k].tier = Tier::of_town(settlements[k].population);
            urban += grown;
        }
        let rural: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&k| !settlements[k].tier.is_urban())
            .collect();
        rescale_rural(settlements, &rural, people.saturating_sub(urban));
    }
    rerank(settlements);
}

/// Makes the seat its realm's largest settlement by trading population and
/// tier with the largest member that outgrows it (people are kept).
fn seat_first(settlements: &mut [Settlement], seat: usize, members: &[usize]) {
    let Some(&big) = members.iter().max_by(|&&a, &&b| {
        settlements[a]
            .population
            .cmp(&settlements[b].population)
            .then(settlements[b].id.cmp(&settlements[a].id))
    }) else {
        return;
    };
    if big != seat && settlements[big].population > settlements[seat].population {
        let (p, t) = (settlements[big].population, settlements[big].tier);
        settlements[big].population = settlements[seat].population;
        settlements[big].tier = settlements[seat].tier;
        settlements[seat].population = p;
        settlements[seat].tier = t;
    }
}

/// Adds up to `want` people to `members`, in proportion to their size, none
/// above `ceiling`; returns how many were placed.
fn grow(settlements: &mut [Settlement], members: &[usize], want: u64, ceiling: u32) -> u64 {
    let mut placed = 0_u64;
    for _ in 0..4 {
        let left = want - placed;
        let room: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&k| settlements[k].population < ceiling)
            .collect();
        let pool: u64 = room
            .iter()
            .map(|&k| u64::from(settlements[k].population))
            .sum();
        if left == 0 || pool == 0 {
            break;
        }
        for &k in &room {
            let s = &mut settlements[k];
            let share = (u64::from(s.population) * left / pool)
                .max(1)
                .min(want - placed);
            let add = share.min(u64::from(ceiling - s.population));
            s.population += u32::try_from(add).unwrap_or(0);
            placed += add;
        }
    }
    placed
}

/// Keeps at most [`MAX_CITIES`] cities in every realm, the seat counting
/// first: a further city becomes a town of `CITY_MIN − 1` people, and its
/// surplus goes to the realm's villages (up to their tier ceiling), then to
/// its towns (up to `CITY_MIN − 1`), and what is left to the seat, so the
/// realm keeps every inhabitant (review 3.3-9). `slot_of[k]` is the seat
/// slot settlement `k` swears to.
pub fn cap(settlements: &mut [Settlement], seats: &[usize], slot_of: &[usize]) {
    for (slot, &seat) in seats.iter().enumerate() {
        let members: Vec<usize> = (0..settlements.len())
            .filter(|&k| slot_of.get(k) == Some(&slot))
            .collect();
        let mut cities: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&k| k != seat && settlements[k].tier == Tier::City)
            .collect();
        cities.sort_by(|&a, &b| {
            settlements[b]
                .population
                .cmp(&settlements[a].population)
                .then(settlements[a].id.cmp(&settlements[b].id))
        });
        let keep = MAX_CITIES - usize::from(settlements[seat].tier == Tier::City);
        let mut surplus = 0_u64;
        for &k in cities.iter().skip(keep) {
            let s = &mut settlements[k];
            surplus += u64::from(s.population - (CITY_MIN - 1));
            s.population = CITY_MIN - 1;
            s.tier = Tier::Town;
        }
        if surplus == 0 {
            continue;
        }
        let of = |settlements: &[Settlement], t: Tier| -> Vec<usize> {
            members
                .iter()
                .copied()
                .filter(|&k| k != seat && settlements[k].tier == t)
                .collect()
        };
        let villages = of(settlements, Tier::Village);
        surplus -= grow(settlements, &villages, surplus, bounds(Tier::Village).1);
        let towns = of(settlements, Tier::Town);
        surplus -= grow(settlements, &towns, surplus, CITY_MIN - 1);
        let s = &mut settlements[seat];
        s.population = s
            .population
            .saturating_add(u32::try_from(surplus).unwrap_or(u32::MAX));
        if s.tier.is_urban() {
            s.tier = Tier::of_town(s.population);
        }
    }
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
