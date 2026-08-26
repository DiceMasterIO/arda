//! Feature 02 R8 measurement probes. Run explicitly, in release:
//! `cargo test -p arda-gen --release --test continent_measures -- --ignored --nocapture`
//! Results are recorded in the feature's spike-report.md, not asserted.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use arda_core::{GenerateConfig, LatitudeBand, SizeKm};
use arda_gen::continent::climate::climate;
use arda_gen::continent::generate_continent;
use arda_gen::continent::hydrology::{extract_rivers, hydrology, river_threshold_km2};

fn default_config() -> GenerateConfig {
    GenerateConfig::new(SizeKm::new(500, 1_000), LatitudeBand::new(35, 55), 15).unwrap()
}

#[test]
#[ignore = "R8 measurement probe, run manually in release"]
fn measure_rainfall() {
    // Sanity band 300-2,500 mm/yr land mean; windward > leeward.
    for seed in [7u64, 42] {
        let g = generate_continent(seed, default_config());
        let c = climate(&g, LatitudeBand::new(35, 55));
        let (w, h) = (g.width(), g.height());
        let (mut land, mut sum) = (0u64, 0u64);
        let (mut wind, mut wn, mut lee, mut ln) = (0u64, 0u64, 0u64, 0u64);
        for y in 0..h {
            for x in 1..w {
                let i = (y * w + x) as usize;
                if g.get(x, y).raw() <= 0 {
                    continue;
                }
                land += 1;
                sum += u64::from(c.rainfall[i]);
                if g.get(x, y).raw() > g.get(x - 1, y).raw() {
                    wind += u64::from(c.rainfall[i]);
                    wn += 1;
                } else {
                    lee += u64::from(c.rainfall[i]);
                    ln += 1;
                }
            }
        }
        println!(
            "seed {seed}: land-mean {} mm/yr, windward {} vs leeward {} mm/yr",
            sum / land.max(1),
            wind / wn.max(1),
            lee / ln.max(1)
        );
    }
}

#[test]
#[ignore = "R8 Horton probe (spike S1 residue), run manually in release"]
fn measure_horton() {
    for seed in [1u64, 7, 42, 99] {
        let g = generate_continent(seed, default_config());
        let c = climate(&g, LatitudeBand::new(35, 55));
        let hy = hydrology(&g, &c);
        let (w, h) = (g.width(), g.height());
        let count = (w * h) as usize;
        let land_km2 = (0..count)
            .filter(|&i| g.get(i as i32 % w, i as i32 / w).raw() > 0)
            .count() as u32;
        let threshold = river_threshold_km2(land_km2);
        let network: Vec<bool> = (0..count)
            .map(|i| {
                hy.catchment_km2[i] >= threshold && g.get(i as i32 % w, i as i32 / w).raw() > 0
            })
            .collect();

        // Strahler over the network, walked high-to-low on the
        // routing surface (test-local: orders are not stored fields).
        let mut order_idx: Vec<(i32, usize)> =
            hy.filled.iter().enumerate().map(|(i, &f)| (f, i)).collect();
        order_idx.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let (mut strahler, mut max_in, mut max_ct) =
            (vec![0u8; count], vec![0u8; count], vec![0u16; count]);
        for &(_, i) in &order_idx {
            if !network[i] {
                continue;
            }
            strahler[i] = if max_in[i] == 0 {
                1
            } else if max_ct[i] >= 2 {
                max_in[i] + 1
            } else {
                max_in[i]
            };
            if let Some(d) = hy.downstream[i] {
                let d = d as usize;
                match strahler[i].cmp(&max_in[d]) {
                    std::cmp::Ordering::Greater => {
                        max_in[d] = strahler[i];
                        max_ct[d] = 1;
                    }
                    std::cmp::Ordering::Equal => max_ct[d] += 1,
                    std::cmp::Ordering::Less => {}
                }
            }
        }

        // Stream count per order = heads of maximal same-order runs:
        // count cells whose downstream has a different order or leaves
        // the network. Sized from the observed maximum order, so no
        // fixed cap can panic.
        let max_order = strahler.iter().copied().max().unwrap_or(0) as usize;
        let mut streams = vec![0u32; max_order + 2];
        for i in 0..count {
            if !network[i] || strahler[i] == 0 {
                continue;
            }
            let ends_run = match hy.downstream[i] {
                None => true,
                Some(d) => !network[d as usize] || strahler[d as usize] != strahler[i],
            };
            if ends_run {
                streams[strahler[i] as usize] += 1;
            }
        }
        let ratios: Vec<String> = (1..)
            .take_while(|&o| o + 1 < streams.len() && streams[o + 1] > 0)
            .map(|o| {
                format!(
                    "N{o}/N{} = {:.2}",
                    o + 1,
                    f64::from(streams[o]) / f64::from(streams[o + 1])
                )
            })
            .collect();
        let rivers = extract_rivers(&g, &hy);
        let biggest = rivers.iter().map(|r| r.catchment_km2).max().unwrap_or(0);
        println!("seed {seed}: threshold {threshold} km\u{b2}, {} rivers, largest {biggest} km\u{b2}, Horton [{}]",
            rivers.len(), ratios.join(", "));
    }
}
