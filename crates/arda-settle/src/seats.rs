//! Realm seats (`logic/08` §realm-seats step 1; `logic/06` step 1; goal 38).
//!
//! How many realms the land holds follows from its people and its habitable
//! land: a realm needs enough people to feed a capital city and its market
//! towns ([`PEOPLE_PER_REALM`]), enough habitable land for a core
//! ([`HABITABLE_PER_REALM`]), and at least [`TOWNS_PER_REALM`] towns to
//! govern. Seats are then chosen among the larger half of the towns so
//! they stand apart and share the land fairly:
//!
//! 1. The largest town is the first seat. Each further seat is the town
//!    that maximises `d² · population`, where `d` is its travel cost to the
//!    nearest seat already chosen over the market lattice (`market.rs`; a
//!    size-weighted farthest-point pick), so seats are large towns spread
//!    over the whole land rather than the largest towns bunched on the best
//!    coast.
//! 2. The seats are then balanced over carrying capacity: the market areas
//!    around the seats (least travel cost, so ranges part them) are weighed
//!    by people and by land, and a seat moves to another town of its own
//!    market area when that brings the realms' shares closer to `1/N`.
//! 3. A market area holding fewer than [`CORE_PEOPLE`] has no real core:
//!    the land then holds one realm fewer and the choice is made again.
//!
//! The partition itself (`realms.rs`) follows the road and terrain cost
//! surface; the market areas here only steer the choice of seats, and give
//! each town its realm for the primate-city rule (`primacy.rs`).

use crate::market::{Lattice, NONE};
use crate::model::Settlement;
use crate::num::dist_m;

/// People per realm: a capital city of 8,000 needs a realm three times its
/// size (assumed, tunable; `logic/06` names 8,000 per seat for a seat
/// that is a town).
pub const PEOPLE_PER_REALM: u64 = 24_000;
/// Habitable hectares (suitability above zero) per realm: 600 km², a core
/// about a day's ride across (assumed, tunable).
pub const HABITABLE_PER_REALM: u64 = 60_000;
/// Towns per realm: the capital and at least two market towns (assumed,
/// tunable; the former `towns / 3` cap).
pub const TOWNS_PER_REALM: usize = 3;
/// People a realm's market area must hold to be a realm: two thirds of
/// [`PEOPLE_PER_REALM`] (assumed, tunable).
pub const CORE_PEOPLE: u64 = PEOPLE_PER_REALM * 2 / 3;
/// Weight of land against people in a market area's share, in quarters:
/// people carry three quarters (they are the carrying capacity the land
/// realised), land one, so an empty mountain still counts for something.
const LAND_WEIGHT: i128 = 1;
/// A seat move must cut the imbalance to this share (per cent) or less.
const MOVE_GAIN_PCT: u128 = 95;
/// Balancing passes at most.
const MAX_PASSES: usize = 12;

/// Number of realms for a population, habitable land and town count.
#[must_use]
pub fn seat_count(population: u64, habitable_cells: u64, towns: usize) -> usize {
    if towns < 2 {
        return 1;
    }
    let by_pop = usize::try_from(population / PEOPLE_PER_REALM).unwrap_or(usize::MAX);
    let by_land = usize::try_from(habitable_cells / HABITABLE_PER_REALM).unwrap_or(usize::MAX);
    by_pop.min(by_land).min(towns / TOWNS_PER_REALM).max(2)
}

fn pos(s: &Settlement) -> (i64, i64) {
    (i64::from(s.cell_x), i64::from(s.cell_y))
}

fn block(lat: &Lattice, s: &Settlement) -> usize {
    lat.block(s.cell_x, s.cell_y)
}

/// Market areas of `seats` (settlement indices): owning slot per block.
fn areas(lat: &Lattice, settlements: &[Settlement], seats: &[usize]) -> Vec<u16> {
    let src: Vec<usize> = seats.iter().map(|&k| block(lat, &settlements[k])).collect();
    lat.flood(&src, None).owner
}

/// Sum of squared deviations of each realm's weighted share (land and
/// people, per million each, weighted by [`LAND_WEIGHT`]) from the fair
/// share `4,000,000 / N`.
fn imbalance(lat: &Lattice, settlements: &[Settlement], owner: &[u16], n: usize) -> u128 {
    let mut land_r = vec![0_u64; n];
    let mut pop_r = vec![0_u64; n];
    for (b, &o) in owner.iter().enumerate() {
        if let Some(l) = land_r.get_mut(usize::from(o)) {
            *l += lat.land[b];
        }
    }
    for s in settlements {
        if let Some(p) = pop_r.get_mut(usize::from(owner[block(lat, s)])) {
            *p += u64::from(s.population);
        }
    }
    let land_t: u64 = land_r.iter().sum::<u64>().max(1);
    let pop_t: u64 = pop_r.iter().sum::<u64>().max(1);
    let fair = 4_000_000 / i128::try_from(n.max(1)).unwrap_or(1);
    (0..n)
        .map(|r| {
            let w = LAND_WEIGHT * i128::from(land_r[r] * 1_000_000 / land_t)
                + (4 - LAND_WEIGHT) * i128::from(pop_r[r] * 1_000_000 / pop_t);
            u128::try_from((w - fair) * (w - fair)).unwrap_or(0)
        })
        .sum()
}

/// Chooses `n` seats among the urban settlements (indices into
/// `settlements`), in no particular order.
#[must_use]
pub fn choose(lat: &Lattice, settlements: &[Settlement], n: usize) -> Vec<usize> {
    let mut urban: Vec<usize> = (0..settlements.len())
        .filter(|&k| settlements[k].tier.is_urban())
        .collect();
    urban.sort_by(|&a, &b| {
        settlements[b]
            .population
            .cmp(&settlements[a].population)
            .then(settlements[a].id.cmp(&settlements[b].id))
    });
    if urban.is_empty() {
        // No towns: the largest settlement is the one seat (logic/08
        // Branches).
        return largest(settlements).into_iter().collect();
    }
    let mut n = n.clamp(1, urban.len());
    loop {
        let seats = pick(lat, settlements, &urban, n);
        let owner = areas(lat, settlements, &seats);
        let mut people = vec![0_u64; seats.len()];
        for s in settlements {
            if let Some(p) = people.get_mut(usize::from(owner[block(lat, s)])) {
                *p += u64::from(s.population);
            }
        }
        // Every realm needs a real core; otherwise the land holds one
        // realm fewer.
        if n <= 2 || people.iter().all(|&p| p >= CORE_PEOPLE) {
            return seats;
        }
        n -= 1;
    }
}

/// `n` seats among `urban` (sorted largest first).
fn pick(lat: &Lattice, settlements: &[Settlement], urban: &[usize], n: usize) -> Vec<usize> {
    // Seats are among the larger towns: the larger half, and at least
    // twice as many as there are seats.
    let keep = urban.len().div_ceil(2).max(2 * n).min(urban.len());
    let urban = &urban[..keep];
    // 1. Size-weighted farthest-point seeding over travel cost; a town
    // the lattice cannot reach from any seat (an island) counts its
    // straight-line distance.
    let mut seats = vec![urban[0]];
    let mut near = lat.flood(&[block(lat, &settlements[urban[0]])], None).cost;
    while seats.len() < n {
        let pick = urban
            .iter()
            .filter(|k| !seats.contains(k))
            .map(|&k| {
                let s = &settlements[k];
                let d = match near[block(lat, s)] {
                    u64::MAX => seats
                        .iter()
                        .map(|&j| {
                            u64::from(dist_m(
                                pos(s).0 - pos(&settlements[j]).0,
                                pos(s).1 - pos(&settlements[j]).1,
                            ))
                        })
                        .min()
                        .unwrap_or(0),
                    d => d,
                };
                (u128::from(d) * u128::from(d) * u128::from(s.population), k)
            })
            .fold(None, |best: Option<(u128, usize)>, c| match best {
                Some(b) if b.0 >= c.0 => Some(b),
                _ => Some(c),
            });
        let Some((_, k)) = pick else { break };
        seats.push(k);
        let f = lat.flood(&[block(lat, &settlements[k])], Some(&near));
        for (m, c) in near.iter_mut().zip(f.cost) {
            *m = (*m).min(c);
        }
    }
    // 2. Balance the market areas over land and people: a seat moves to a
    // town of its own area when that cuts the imbalance by a twentieth.
    let n = seats.len();
    let mut owner = areas(lat, settlements, &seats);
    let mut score = imbalance(lat, settlements, &owner, n);
    for _ in 0..MAX_PASSES {
        let mut moved = false;
        for slot in 0..n {
            let mut best: Option<(u128, usize, Vec<u16>)> = None;
            let own = u16::try_from(slot).unwrap_or(u16::MAX);
            for &k in urban.iter().filter(|k| !seats.contains(k)) {
                if owner[block(lat, &settlements[k])] != own {
                    continue;
                }
                let mut trial = seats.clone();
                trial[slot] = k;
                let o = areas(lat, settlements, &trial);
                let sc = imbalance(lat, settlements, &o, n);
                if best.as_ref().is_none_or(|b| sc < b.0) {
                    best = Some((sc, k, o));
                }
            }
            if let Some((sc, k, o)) = best {
                if sc * 100 <= score * MOVE_GAIN_PCT {
                    seats[slot] = k;
                    score = sc;
                    owner = o;
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
    seats
}

fn largest(settlements: &[Settlement]) -> Option<usize> {
    (0..settlements.len()).min_by(|&a, &b| {
        settlements[b]
            .population
            .cmp(&settlements[a].population)
            .then(settlements[a].id.cmp(&settlements[b].id))
    })
}

/// Seat slot every settlement's block falls to over the lattice; a
/// settlement the lattice cannot reach goes to the nearest seat in a
/// straight line.
#[must_use]
pub fn allegiance(lat: &Lattice, settlements: &[Settlement], seats: &[usize]) -> Vec<usize> {
    let owner = areas(lat, settlements, seats);
    settlements
        .iter()
        .map(|s| match owner[block(lat, s)] {
            NONE => nearest(
                pos(s),
                &seats
                    .iter()
                    .map(|&k| pos(&settlements[k]))
                    .collect::<Vec<_>>(),
            ),
            o => usize::from(o),
        })
        .collect()
}

/// Slot of the nearest seat to `p` in a straight line; ties go to the lower
/// slot.
#[must_use]
pub fn nearest(p: (i64, i64), seats: &[(i64, i64)]) -> usize {
    let mut best = (u32::MAX, 0);
    for (r, &q) in seats.iter().enumerate() {
        let d = dist_m(p.0 - q.0, p.1 - q.1);
        if d < best.0 {
            best = (d, r);
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Tier;

    fn town(id: u64, x: u32, y: u32, population: u32) -> Settlement {
        Settlement::bare(id, x, y, Tier::of_town(population), population)
    }

    #[test]
    fn realm_count_scales_with_people_land_and_towns() {
        assert_eq!(seat_count(1_000_000, 10_000_000, 1), 1);
        assert_eq!(seat_count(1000, 1000, 2), 2);
        assert_eq!(seat_count(92_000, 400_000, 9), 3);
        assert_eq!(seat_count(92_000, 130_000, 30), 2);
        assert_eq!(seat_count(200_000, 1_000_000, 16), 5);
        assert_eq!(seat_count(200_000, 1_000_000, 40), 8);
    }

    #[test]
    fn seats_spread_instead_of_bunching_on_the_best_coast() {
        // Four big towns in the north-west corner, two smaller ones far
        // south and east, on a 200 × 400 strip of land.
        let mut s = vec![
            town(1, 10, 10, 9000),
            town(2, 30, 10, 4500),
            town(3, 10, 30, 3000),
            town(4, 30, 30, 2250),
            town(5, 150, 350, 1800),
            town(6, 100, 200, 1500),
        ];
        // Villages spread evenly over the strip.
        for y in (20..400).step_by(20) {
            for x in (20..200).step_by(20) {
                let id = u64::try_from(s.len() + 1).unwrap();
                s.push(Settlement::bare(id, x, y, Tier::Village, 400));
            }
        }
        let land = Lattice::new(200, 400, |_| true, |_| 0);
        let mut seats = choose(&land, &s, 3);
        seats.sort_unstable();
        assert!(seats.contains(&4) && seats.contains(&5), "{seats:?}");
        assert_eq!(
            seats.iter().filter(|&&k| k < 4).count(),
            1,
            "one seat in the north-west: {seats:?}"
        );
    }

    #[test]
    fn choice_is_deterministic_and_nearest_breaks_ties_low() {
        let s = vec![town(1, 0, 0, 5000), town(2, 10, 0, 5000)];
        let land = Lattice::new(20, 20, |_| true, |_| 0);
        assert_eq!(choose(&land, &s, 2), choose(&land, &s, 2));
        assert_eq!(nearest((5, 0), &[(0, 0), (10, 0)]), 0);
    }
}
