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
                // A parapet stops the step anyway.
                let (ex, ey, axis) = match (dx, dy) {
                    (1, 0) => (nx, y, arda_tactical::layout::EdgeAxis::Vertical),
                    (-1, 0) => (x, y, arda_tactical::layout::EdgeAxis::Vertical),
                    (0, 1) => (x, ny, arda_tactical::layout::EdgeAxis::Horizontal),
                    _ => (x, y, arda_tactical::layout::EdgeAxis::Horizontal),
                };
                let walled = c.rules.edges.iter().any(|e| {
                    (i64::from(e.x), i64::from(e.y), e.axis) == (ex, ey, axis) && e.blocks_movement
                });
                if walled {
                    continue;
                }
                ends += 1;
                let step = i32::from(deck) - i32::from(sq.elevation_ft);
                if step.abs() >= 10 {
                    worst.push(format!(
                        "crossing {}: deck {deck} ft, bank {} ft ({:?} {:?} {} at {x},{y} step {dx},{dy})",
                        b.id,
                        sq.elevation_ft,
                        c.owners[j],
                        c.rules.squares[j].ext,
                        sq.ground
                    ));
                }
            }
        }
    }
    eprintln!("{ends} deck ends, {} climbs", worst.len());
    assert!(ends > 0, "no bridge deck met a bank");
    // Two crossings laid over one another on a skewed river can still
    // leave an abutment of one beside the deck of the other (open-items).
    assert!(worst.len() * 50 <= ends, "{}", worst.join("\n"));
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

/// Review 3.5-9: a sea ferry is laid across all of its open water, shore
/// to shore, or, too wide to rope, with a landing and a boat on each
/// shore; never cut to a pad of a few squares.
#[test]
fn sea_ferries_span_their_water() {
    let (world, society, pipeline) = open();
    let library = lib();
    let ferries: Vec<_> = world
        .files
        .roads
        .crossings
        .iter()
        .filter(|c| c.kind == CrossingKind::Ferry && c.water != "river")
        .collect();
    assert!(!ferries.is_empty());
    // A ferry no road comes within 150 m of has no way to land (the
    // ways' rule): it stays an orphan in the report.
    #[allow(clippy::cast_precision_loss)]
    let reached = |f: &arda_settle::crossings::Crossing| {
        let q = [f.x_m as f64, f.y_m as f64];
        world
            .files
            .roads
            .roads
            .iter()
            .flat_map(|r| &r.segments)
            .any(|seg| {
                seg.windows(2).any(|w| {
                    let (a, b) = (
                        [w[0][0] as f64, w[0][1] as f64],
                        [w[1][0] as f64, w[1][1] as f64],
                    );
                    let d = [b[0] - a[0], b[1] - a[1]];
                    let l2 = d[0] * d[0] + d[1] * d[1];
                    let t = if l2 > 0.0 {
                        (((q[0] - a[0]) * d[0] + (q[1] - a[1]) * d[1]) / l2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    (q[0] - a[0] - d[0] * t).hypot(q[1] - a[1] - d[1] * t) <= 150.0
                })
            })
    };
    let ferries: Vec<_> = ferries.into_iter().filter(|f| reached(f)).collect();
    assert!(!ferries.is_empty());
    let mut report = Vec::new();
    for f in ferries {
        // Settle stops a ferry's road at a cell centre short of the water:
        // look around the record and around every road end near it.
        let mut centres = vec![cell_at(f.x_m, f.y_m)];
        for r in &world.files.roads.roads {
            for seg in &r.segments {
                for p in [seg.first(), seg.last()].into_iter().flatten() {
                    if (p[0] - f.x_m).abs() <= 600 && (p[1] - f.y_m).abs() <= 600 {
                        centres.push(cell_at(p[0], p[1]));
                    }
                }
            }
        }
        centres.sort_unstable();
        centres.dedup();
        let (mut landings, mut rope, mut boats) = (0, 0, 0);
        for (cx, cy) in centres {
            let win = ((cx - 1) * 64, (cy - 1) * 64, 192, 192);
            let c = pipeline.window(win, &library, Some(&society)).unwrap();
            let feature = |i: usize| {
                c.rules.squares[i]
                    .ext
                    .as_ref()
                    .and_then(|e| e.get("feature"))
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            };
            landings += (0..c.rules.squares.len())
                .filter(|&i| feature(i).as_deref() == Some("landing"))
                .count();
            let count = |want: &str| {
                c.layout
                    .placements
                    .iter()
                    .filter(|p| matches!(&p.asset, arda_tactical::layout::AssetRef::Id(id) if id == want))
                    .count()
            };
            rope += count("prop.ferry_rope");
            boats += count("prop.ferry_boat");
        }
        report.push(format!(
            "ferry {}: {landings} landing squares, {rope} rope, {boats} boats",
            f.id
        ));
        assert!(landings > 0, "ferry {}: no landing", f.id);
        assert!(boats > 0, "ferry {}: no boat", f.id);
        // A roped ferry crosses its water; a sea route too wide to rope
        // lands on each shore instead.
        assert!(
            rope == 0 || rope >= 8,
            "ferry {}: a {rope}-square rope is a pad",
            f.id
        );
    }
    eprintln!("{}", report.join("\n"));
}
