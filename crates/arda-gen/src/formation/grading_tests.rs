use super::super::drainage::{fill, fill_local, receiver_index, receivers, upstream_order, SELF};
use super::*;

/// A 201 × 201 closed basin: a flat floor at 1 m inside a 5 m rim, with
/// its spill (fixed, below sea) at the middle of the east side.
fn basin() -> (Vec<i32>, Vec<u8>, usize, usize) {
    let (w, h) = (201, 201);
    let mut z = vec![1_000_i32; w * h];
    let mut flags = vec![0_u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                z[i] = 5_000;
                flags[i] = FIXED;
            }
        }
    }
    z[100 * w + w - 1] = -1_000;
    (z, flags, w, h)
}

fn drains(z: &[i32], flags: &[u8], w: usize, h: usize) -> bool {
    (0..w * h).all(|i| {
        flags[i] & FIXED != 0 || (0..8).any(|k| neighbour(i, w, h, k).is_some_and(|j| z[j] < z[i]))
    })
}

/// Longest run of identical steepest-descent steps along any path that
/// carries at least 200 cells of area.
fn straightest_run(z: &[i32], flags: &[u8], w: usize, h: usize) -> usize {
    let mut rcv = vec![0_u8; w * h];
    receivers(z, w, h, flags, 0, 0, &mut rcv);
    let (mut indeg, mut order) = (vec![0_u8; w * h], vec![0_u32; w * h]);
    upstream_order(&rcv, w, h, &mut indeg, &mut order);
    let mut area = vec![1_u32; w * h];
    for &i in &order {
        let i = i as usize;
        if rcv[i] != SELF {
            area[receiver_index(i, w, h, rcv[i])] += area[i];
        }
    }
    let mut best = 0;
    for i in 0..w * h {
        if area[i] < 200 || rcv[i] == SELF {
            continue;
        }
        let (mut run, mut cur) = (0, i);
        while rcv[cur] == rcv[i] {
            run += 1;
            cur = receiver_index(cur, w, h, rcv[cur]);
        }
        best = best.max(run);
    }
    best
}

/// Review 3.1-3: grid-step grading makes square rings, and flow over the
/// flat runs in straight axis-parallel lines to the spill's row. Recipe
/// 9's weighted geodesic drains the same flat along wandering paths.
#[test]
fn graded_fill_flats_drain_without_long_straight_lines() {
    let (mut rings, flags, w, h) = basin();
    let (mut next, mut closed) = (vec![0; w * h], vec![0; w * h]);
    fill(&mut rings, w, h, &flags, 2, &mut next, &mut closed).unwrap();
    let (mut graded, ..) = basin();
    fill_graded(&mut graded, w, h, &flags, 2, 7, &mut next, &mut closed).unwrap();
    assert!(drains(&rings, &flags, w, h) && drains(&graded, &flags, w, h));
    let (before, after) = (
        straightest_run(&rings, &flags, w, h),
        straightest_run(&graded, &flags, w, h),
    );
    eprintln!("straightest run: rings {before}, graded {after}");
    assert!(before >= 90, "ring grading runs straight: {before}");
    assert!(
        after * 4 < before,
        "graded paths wander: {after} vs {before}"
    );
    // The floor stays a flat: the graded surface is within a metre.
    let top = graded
        .iter()
        .zip(&flags)
        .filter(|(_, &f)| f == 0)
        .map(|(&v, _)| v)
        .max()
        .unwrap();
    assert!(top < 2_000, "graded floor top {top} mm");
}

/// The local fill (in-level drains) grades the same way when given a seed.
#[test]
fn graded_local_fill_matches_the_global_drainage_guarantee() {
    let (mut z, flags, w, h) = basin();
    let (mut stamp, mut closed) = (vec![0; w * h], vec![0; w * h]);
    fill_local(
        &mut z,
        w,
        h,
        &flags,
        2,
        (&mut stamp, &mut closed),
        true,
        Some(7),
    )
    .unwrap();
    assert!(drains(&z, &flags, w, h));
    let run = straightest_run(&z, &flags, w, h);
    assert!(run < 30, "local graded paths wander: {run}");
}

#[test]
fn step_costs_are_euclidean_on_average_and_at_least_one_millimetre() {
    let (mut card, mut diag) = (0_i64, 0_i64);
    for i in 0..10_000 {
        card += i64::from(step_mm(3, 2, i, 100, 0));
        diag += i64::from(step_mm(3, 2, i, 100, 5));
        assert!(step_mm(3, 1, i, 100, 0) >= 1);
    }
    let ratio = diag * 1_000 / card;
    assert!(
        (1_350..=1_480).contains(&ratio),
        "diagonal/cardinal {ratio}‰"
    );
    assert!((19_000..=21_000).contains(&card), "mean cardinal {card}");
}
