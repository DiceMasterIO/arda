//! Crossings on the real seed-42 MICRO world with every society overlay:
//! bridge decks stand at the level of their banks and the scene walks
//! them there (review 3.6-6).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod support;

use arda_settle::crossings::CrossingKind;
use support::world::{cell_at, lib, open};

/// The deck level a composed square states, if it is a bridge deck.
fn deck_ft(c: &arda_scene::RulesCell) -> Option<i16> {
    if c.deck != Some(true) {
        return None;
    }
    let ext = c.ext.as_ref()?;
    if ext.get("feature").and_then(|v| v.as_str()) != Some("bridge") {
        return None;
    }
    ext.get("deck_elevation_ft")
        .and_then(serde_json::Value::as_i64)
        .and_then(|v| i16::try_from(v).ok())
}

#[test]
fn bridge_decks_meet_their_banks_and_the_scene_stands_on_them() {
    let (world, society, pipeline) = open();
    let library = lib();
    let bridges: Vec<_> = world
        .files
        .roads
        .crossings
        .iter()
        .filter(|c| c.kind == CrossingKind::Bridge)
        .take(40)
        .collect();
    assert!(!bridges.is_empty());
    let mut ends = 0;
    let mut worst = Vec::new();
    for b in bridges {
        let (cx, cy) = cell_at(b.x_m, b.y_m);
        let c = pipeline
            .window((cx * 64, cy * 64, 64, 64), &library, Some(&society))
            .unwrap();
        let scene = arda_scene::build_scene(&c.layout, &library, 1, Some(&c.rules)).unwrap();
        let w = 64_i64;
        for (i, cell) in c.rules.squares.iter().enumerate() {
            let Some(deck) = deck_ft(cell) else { continue };
            assert_eq!(
                scene.elevation_ft.0[i], deck,
                "crossing {}: the scene stands under the deck at square {i}",
                b.id
            );
            let (x, y) = (i64::try_from(i).unwrap() % w, i64::try_from(i).unwrap() / w);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if !(0..w).contains(&nx) || !(0..w).contains(&ny) {
                    continue;
                }
                let j = usize::try_from(ny * w + nx).unwrap();
                let sq = &c.layout.squares[j];
                if sq.water_depth_ft > 0 || c.rules.squares[j].deck == Some(true) {
                    continue;
                }
                ends += 1;
                let step = i32::from(deck) - i32::from(sq.elevation_ft);
                if step.abs() >= 10 {
                    worst.push(format!(
                        "crossing {}: deck {deck} ft, bank {} ft",
                        b.id, sq.elevation_ft
                    ));
                }
            }
        }
    }
    eprintln!("{ends} deck ends, {} climbs", worst.len());
    assert!(ends > 0, "no bridge deck met a bank");
    assert!(worst.is_empty(), "{}", worst.join("\n"));
}

/// Review 3.5-11: the ways' dressing that stands beside the ground they
/// claim (ford marker posts in the shallows, reeds on the banks,
/// milestones past the verge) survives composition wherever no higher
/// layer took its square.
#[test]
fn ford_posts_bank_reeds_and_milestones_survive() {
    use arda_blocks::{OverlayCtx, Overlays, Owner};
    use arda_tactical::layout::AssetRef;
    let (world, society, pipeline) = open();
    let library = lib();
    let dressing = ["prop.marker_post", "veg.reeds", "prop.milestone"];
    let mut cells: Vec<(i64, i64)> = world
        .files
        .roads
        .crossings
        .iter()
        .filter(|c| c.kind == CrossingKind::Ford)
        .take(10)
        .map(|c| cell_at(c.x_m, c.y_m))
        .collect();
    cells.sort_unstable();
    cells.dedup();
    let (mut laid, mut kept) = (0, 0);
    let mut lost = Vec::new();
    for (cx, cy) in cells {
        let win = (cx * 64, cy * 64, 64, 64);
        let composed = pipeline.window(win, &library, Some(&society)).unwrap();
        let base = pipeline.window(win, &library, None).unwrap().layout;
        let ctx = OverlayCtx {
            gsx0: win.0,
            gsy0: win.1,
            width: 64,
            height: 64,
            seed: pipeline.seed(),
            base: &base,
            library: &library,
        };
        let Some(ways) = society.ways(&ctx).unwrap() else {
            continue;
        };
        for p in &ways.layout.placements {
            let AssetRef::Id(id) = &p.asset else { continue };
            if !dressing.contains(&id.as_str())
                || !(0.0..64.0).contains(&p.x)
                || !(0.0..64.0).contains(&p.y)
            {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = p.y.floor() as usize * 64 + p.x.floor() as usize;
            if matches!(
                composed.owners[i],
                Owner::Town | Owner::Fields | Owner::Croft
            ) {
                continue;
            }
            laid += 1;
            let near = |q: &arda_tactical::layout::Placement| {
                q.asset == p.asset && (q.x - p.x).abs() < 0.02 && (q.y - p.y).abs() < 0.02
            };
            if composed.layout.placements.iter().any(near) {
                kept += 1;
            } else {
                lost.push(format!("{id} at {:.2},{:.2} of {cx},{cy}", p.x, p.y));
            }
        }
    }
    eprintln!("{kept} of {laid} kept");
    assert!(laid > 0, "no dressing laid");
    assert!(lost.is_empty(), "{} lost:\n{}", lost.len(), lost.join("\n"));
}
