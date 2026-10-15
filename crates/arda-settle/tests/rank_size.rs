//! Rank-size is an outcome, not a formula (review 3.3-12; logic/08
//! Invariants 4): towns are sized by their hinterland and their site, so
//! equal towns come out equal and the rank-size check can fail.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::model::{Settlement, Tier};
use arda_settle::stats::rank_size_fit;
use arda_settle::{tags, townsize};

/// Five towns 40 km apart in a row, each with the same ring of villages.
fn row(harbour_at: Option<usize>) -> Vec<Settlement> {
    let mut s = Vec::new();
    let mut id = 1;
    for t in 0..5_u32 {
        let mut town = Settlement::bare(id, 400 * t + 200, 200, Tier::Town, 1000);
        if harbour_at == Some(usize::try_from(t).unwrap()) {
            town.tag_bits = tags::HARBOUR;
        }
        s.push(town);
        id += 1;
        for v in 0..8_u32 {
            s.push(Settlement::bare(
                id,
                400 * t + 150 + 12 * v,
                260,
                Tier::Village,
                400,
            ));
            id += 1;
        }
    }
    s
}

#[test]
fn equal_towns_come_out_equal_so_rank_size_is_not_imposed() {
    let mut s = row(None);
    townsize::size(&mut s, 15_000);
    let towns: Vec<u32> = s
        .iter()
        .filter(|x| x.tier.is_urban())
        .map(|x| x.population)
        .collect();
    assert_eq!(towns.len(), 5);
    let (lo, hi) = (towns.iter().min().unwrap(), towns.iter().max().unwrap());
    assert!(hi - lo <= 1, "{towns:?}");
    let (slope, _) = rank_size_fit(&s);
    assert!(slope > -0.05, "a flat list has no rank-size slope: {slope}");
    assert_eq!(towns.iter().map(|&p| u64::from(p)).sum::<u64>(), 15_000);
}

#[test]
fn a_harbour_draws_more_than_its_hinterland() {
    let mut s = row(Some(2));
    townsize::size(&mut s, 15_000);
    let towns: Vec<u32> = s
        .iter()
        .filter(|x| x.tier.is_urban())
        .map(|x| x.population)
        .collect();
    assert!(towns[2] > towns[0] && towns[2] > towns[4], "{towns:?}");
}
