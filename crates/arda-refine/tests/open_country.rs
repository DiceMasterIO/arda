//! Open country at 5-ft scale (goals 19, 20 and 43): species follow the
//! climate, feature counts stay within bounds, scatter and trails are global
//! functions that neighbouring blocks agree on, and trails run on unbroken
//! across block edges.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs
)]

mod common;
use arda_core::TempCentiC;
use arda_refine::context::Ctx;
use arda_refine::scatter::{scatter, Item, Kind};
use arda_refine::trails::{edge_way, trails, Way, TRAIL};
use arda_refine::{refine, Block, CellKey, GridSource};

/// The synthetic world with every cell's mean temperature set to `c` °C.
fn climate(seed: u64, c: i16) -> GridSource {
    let mut src = common::world(seed);
    for y in 0..i64::from(common::W) {
        for x in 0..i64::from(common::W) {
            if let Some(cell) = src.cell_mut(CellKey::new(x, y)) {
                cell.temperature = TempCentiC::new(c * 100);
            }
        }
    }
    src
}

fn trees(b: &Block) -> impl Iterator<Item = &Item> {
    b.items
        .iter()
        .filter(|i| matches!(i.kind, Kind::TreeLarge | Kind::TreeSmall))
}

fn share(blocks: &[Block], f: impl Fn(&str) -> bool) -> f64 {
    let all: Vec<&Item> = blocks.iter().flat_map(trees).collect();
    all.iter().filter(|i| f(i.asset)).count() as f64 / all.len().max(1) as f64
}

#[test]
fn species_follow_the_climate_and_the_water() {
    let cells = [CellKey::new(2, 2), CellKey::new(3, 3), CellKey::new(6, 4)];
    let run = |t: i16| -> Vec<Block> {
        let src = climate(42, t);
        cells.iter().map(|&c| refine(&src, c).unwrap()).collect()
    };
    let conifer = |id: &str| matches!(id, "veg.tree_pine" | "veg.tree_spruce");
    let (warm, cold, line) = (run(12), run(3), run(0));
    let (w, c) = (share(&warm, conifer), share(&cold, conifer));
    assert!(
        w < 0.15 && c > 0.55,
        "conifer share warm {w:.2}, cold {c:.2}"
    );
    let stunted = |id: &str| matches!(id, "veg.tree_stunted" | "veg.tree_dead");
    let (s0, s1) = (share(&warm, stunted), share(&line, stunted));
    assert!(
        s0 < 0.05 && s1 > 0.3,
        "stunted warm {s0:.2}, tree line {s1:.2}"
    );
    // Along the river, willows and alders.
    let src = common::world(42);
    let river: Vec<Block> = (3..9)
        .map(|x| refine(&src, CellKey::new(x, common::RIVER_ROW)).unwrap())
        .collect();
    let wet = |id: &str| matches!(id, "veg.tree_willow" | "veg.tree_alder");
    // Trees standing within two squares of the water.
    let (mut bank, mut by) = (0, 0);
    for b in &river {
        for t in trees(b) {
            let (i, j) = (
                (t.x.floor() as i64 - b.origin.0),
                (t.y.floor() as i64 - b.origin.1),
            );
            let near = (-2..=2).any(|dy: i64| {
                (-2..=2).any(|dx: i64| {
                    let (x, y) = (i + dx, j + dy);
                    (0..64).contains(&x)
                        && (0..64).contains(&y)
                        && b.depth_ft[(y * 64 + x) as usize] > 0
                })
            });
            if near {
                bank += 1;
                by += usize::from(wet(t.asset));
            }
        }
    }
    let riparian = by as f64 / bank.max(1) as f64;
    assert!(
        bank >= 10 && riparian > 0.4,
        "{by} of {bank} bank trees riparian"
    );
    assert!(
        share(&warm, wet) < 0.1,
        "dry forest {:.2}",
        share(&warm, wet)
    );
}

#[test]
fn snow_lies_in_cold_shade_not_in_warm_country() {
    let snow = |src: &GridSource| -> usize {
        common::ROCK
            .iter()
            .map(|&(x, y)| {
                let b = refine(src, CellKey::new(x, y)).unwrap();
                b.ground.iter().filter(|g| **g == "snow").count()
            })
            .sum()
    };
    assert_eq!(snow(&common::world(42)), 0, "no snow at 10 °C");
    let cold = climate(42, 1);
    assert!(snow(&cold) > 40, "snow patches at 1 °C");
    // On the cold hill the snow sits on its north-facing side: count snow
    // where the ground rises southward against where it falls.
    let (mut north, mut south) = (0, 0);
    for &(x, y) in &common::ROCK {
        let b = refine(&cold, CellKey::new(x, y)).unwrap();
        for j in 1..63 {
            for i in 0..64 {
                if b.ground[j * 64 + i] != "snow" {
                    continue;
                }
                let rise = b.elevation_ft[(j + 1) * 64 + i] - b.elevation_ft[(j - 1) * 64 + i];
                if rise > 0 {
                    north += 1;
                } else if rise < 0 {
                    south += 1;
                }
            }
        }
    }
    assert!(north > 2 * south, "north {north}, south {south}");
}

#[test]
fn feature_counts_stay_within_bounds() {
    let src = common::world(42);
    let mut trail_squares = 0;
    let mut land = 0;
    for y in (0..common::SEA_ROW).step_by(2) {
        for x in (0..i64::from(common::W)).step_by(2) {
            let b = refine(&src, CellKey::new(x, y)).unwrap();
            let count = |k: Kind| b.items.iter().filter(|i| i.kind == k).count();
            let outcrops = count(Kind::Outcrop);
            let low = count(Kind::Low);
            let large = count(Kind::TreeLarge);
            assert!(outcrops <= 12, "{x},{y}: {outcrops} outcrops");
            // A closed stand packs a trunk every ~4 square-squares so the
            // art's 2-3 square crowns overlap (biome pass, v0.7.x).
            assert!(large <= 1300, "{x},{y}: {large} large trees");
            assert!(low <= 900, "{x},{y}: {low} low plants");
            let dry = b.depth_ft.iter().filter(|d| **d == 0).count();
            if dry > 3000 && !common::ROCK.contains(&(x, y)) {
                assert!(low >= 20, "{x},{y}: only {low} low plants");
            }
            trail_squares += b.ground.iter().filter(|g| **g == TRAIL).count();
            land += dry;
        }
    }
    let share = trail_squares as f64 / land as f64;
    assert!((0.002..0.05).contains(&share), "trail share {share:.4}");
}

/// The scatter of `cell`'s block over a strip, from that block's own data.
fn strip(src: &GridSource, cell: CellKey, r: (f64, f64, f64, f64)) -> Vec<Item> {
    let ctx = Ctx::gather(src, cell).unwrap();
    let pieces = arda_refine::rivers::pieces(&ctx);
    let (x0, y0) = (cell.x * 64, cell.y * 64);
    let mut phys = arda_refine::terrain::physical(&ctx, &pieces, x0 - 8, y0 - 8, 80);
    arda_refine::terrain::slopes(&mut phys);
    let shape = arda_refine::shape::shapes(&phys, x0 - 2, y0 - 2, 68);
    scatter(&ctx, &phys, &shape, &trails(&ctx), r)
}

#[test]
fn neighbours_agree_on_scatter_and_trails_across_the_seam() {
    for (seed, a) in [
        (42, CellKey::new(4, 3)),
        (9, CellKey::new(9, 6)),
        (5, CellKey::new(6, 9)),
    ] {
        let src = common::world(seed);
        for b in [a.offset(1, 0), a.offset(0, 1)] {
            // The two-square halo strip either side of the shared edge.
            let r = if b.x > a.x {
                let x = (b.x * 64) as f64;
                (x - 2.0, (a.y * 64) as f64, x + 2.0, (a.y * 64 + 64) as f64)
            } else {
                let y = (b.y * 64) as f64;
                ((a.x * 64) as f64, y - 2.0, (a.x * 64 + 64) as f64, y + 2.0)
            };
            assert_eq!(strip(&src, a, r), strip(&src, b, r), "{a:?} {b:?}");
            let (ta, tb) = (
                trails(&Ctx::gather(&src, a).unwrap()),
                trails(&Ctx::gather(&src, b).unwrap()),
            );
            // Every square of the two cells both see.
            for c in [a, b] {
                for y in c.y * 64..c.y * 64 + 64 {
                    for x in c.x * 64..c.x * 64 + 64 {
                        assert_eq!(ta.at(x, y), tb.at(x, y), "trail at {x},{y}");
                    }
                }
            }
        }
    }
}

#[test]
fn trails_cross_block_edges_without_a_break() {
    let src = common::world(42);
    let mut crossings = 0;
    for y in 0..common::SEA_ROW - 1 {
        for x in 0..i64::from(common::W) - 1 {
            let a = CellKey::new(x, y);
            let ctx = Ctx::gather(&src, a).unwrap();
            for b in [a.offset(1, 0), a.offset(0, 1)] {
                if edge_way(&ctx, a, b) == Way::None {
                    continue;
                }
                let (ba, bb) = (refine(&src, a).unwrap(), refine(&src, b).unwrap());
                // Squares facing each other across the edge.
                let pairs: Vec<(usize, usize)> = (0..64)
                    .map(|k| {
                        if b.x > a.x {
                            (k * 64 + 63, k * 64)
                        } else {
                            (63 * 64 + k, k)
                        }
                    })
                    .collect();
                let met = pairs.iter().any(|&(i, j)| {
                    let open = |bl: &Block, s: usize| bl.ground[s] == TRAIL || bl.depth_ft[s] > 0;
                    (ba.ground[i] == TRAIL || bb.ground[j] == TRAIL) && open(&ba, i) && open(&bb, j)
                });
                assert!(met, "trail {a:?} → {b:?} breaks at the edge");
                crossings += 1;
            }
        }
    }
    assert!(crossings >= 5, "only {crossings} trail crossings");
}

#[test]
fn open_country_is_deterministic() {
    for c in [CellKey::new(6, 10), CellKey::new(2, 10), CellKey::new(4, 3)] {
        let a = refine(&common::world(42), c).unwrap();
        let b = refine(&common::world(42), c).unwrap();
        assert_eq!(a, b);
    }
}
