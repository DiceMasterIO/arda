//! The fields layer reads the ways' roads and the refined rivers (review
//! 3.5-10): on the real seed-42 MICRO world, a square the fields leave as
//! a gravelled or cobbled road is a road the ways lay, and a watermill
//! stands on the bank of the river the map draws.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    missing_docs
)]

mod support;

use arda_blocks::{OverlayCtx, Overlays};
use arda_ids::{LandUse, RoadClass};
use support::world::{lib, open, world_dir};

fn feature(rules: &arda_scene::RulesSidecar, i: usize) -> String {
    rules.squares[i]
        .ext
        .as_ref()
        .and_then(|e| e.get("feature"))
        .and_then(|v| v.as_str())
        .unwrap_or("none")
        .to_owned()
}

#[test]
fn fields_roads_are_the_ways_roads_and_mills_stand_on_the_river() {
    let (world, society, pipeline) = open();
    let library = lib();
    let soc = world_dir().join("society");
    let (w, _, codes, _) = arda_settle::output::read_landuse(&soc.join("landuse.bin")).unwrap();
    let (_, _, roads) = arda_settle::output::read_road_map(&soc.join("roads.bin")).unwrap();
    let cell = |i: usize| ((i % w) as i64, (i / w) as i64);
    // Mill cells a refined river runs through (settle sites some mills by
    // a world river the refined map moves a cell away).
    let river_in = |(cx, cy): (i64, i64)| {
        (-1..=1).any(|dy| {
            (-1..=1).any(|dx| {
                !arda_refine::water::cell_channels(
                    &world.src,
                    arda_refine::CellKey::new(cx + dx, cy + dy),
                )
                .unwrap()
                .is_empty()
            })
        })
    };
    let mills: Vec<(i64, i64)> = (0..codes.len())
        .filter(|&i| codes[i] == LandUse::Mill.code())
        .map(cell)
        .filter(|&c| river_in(c))
        .take(6)
        .collect();
    let farm_roads: Vec<(i64, i64)> = (0..codes.len())
        .filter(|&i| {
            codes[i] == LandUse::Field.code()
                && matches!(
                    RoadClass::from_code(roads[i]),
                    Some(RoadClass::Road | RoadClass::Highway)
                )
        })
        .map(cell)
        .step_by(7)
        .take(6)
        .collect();
    assert!(!mills.is_empty() && !farm_roads.is_empty());
    let (mut paved, mut phantom) = (0, 0);
    let mut far_mills = Vec::new();
    let mut mills_seen = 0;
    let mut hist: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (cx, cy) in mills.iter().chain(&farm_roads).copied() {
        let win = (cx * 64, cy * 64, 64_u32, 64_u32);
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
        let fields = society.fields(&ctx).unwrap().unwrap();
        let ways = society.ways(&ctx).unwrap();
        for i in 0..64 * 64 {
            let g = fields.layout.squares[i].ground.as_str();
            if fields.owned[i] || !matches!(g, "gravel" | "cobbles") {
                continue;
            }
            paved += 1;
            let f = ways
                .as_ref()
                .map_or("none".to_owned(), |l| feature(&l.rules, i));
            if !matches!(
                f.as_str(),
                "road" | "ruts" | "bridge" | "abutment" | "ford" | "landing"
            ) {
                phantom += 1;
                *hist.entry(f).or_insert(0) += 1;
            }
        }
        // The mill house's nearest square to the refined water, both read
        // over the cell grown by 32 squares: the bank a mill takes may lie
        // in the next cell.
        let wwin = (win.0 - 32, win.1 - 32, 128_u32, 128_u32);
        let wide = pipeline.window(wwin, &library, None).unwrap().layout;
        let wctx = OverlayCtx {
            gsx0: wwin.0,
            gsy0: wwin.1,
            width: 128,
            height: 128,
            seed: pipeline.seed(),
            base: &wide,
            library: &library,
        };
        let wfields = society.fields(&wctx).unwrap().unwrap();
        let house: Vec<(i64, i64)> = (0..128 * 128)
            .filter(|&i| {
                wfields.rules.squares[i]
                    .ext
                    .as_ref()
                    .and_then(|e| e.get("building_function"))
                    .and_then(|v| v.as_str())
                    == Some("mill")
            })
            .map(|i| ((i % 128) as i64, (i / 128) as i64))
            .collect();
        if house.is_empty() {
            continue;
        }
        mills_seen += 1;
        let water: Vec<(i64, i64)> = (0..128 * 128)
            .filter(|&i| wide.squares[i].water_depth_ft > 0)
            .map(|i| ((i % 128) as i64, (i / 128) as i64))
            .collect();
        let d = house
            .iter()
            .flat_map(|h| {
                water
                    .iter()
                    .map(move |w| (h.0 - w.0).abs().max((h.1 - w.1).abs()))
            })
            .min()
            .unwrap_or(i64::MAX);
        // The house stands three squares back from its bank, behind the
        // leat (`arda_fields::compounds::mill`); settle's straight river
        // left mills 16 squares off.
        if d > 8 {
            far_mills.push(format!(
                "mill at cell {cx},{cy} stands {d} squares from the water"
            ));
        }
    }
    eprintln!("{phantom} of {paved} fields road squares have no carriageway; {mills_seen} mills; {hist:?}");
    assert!(paved > 0);
    assert!(
        phantom * 50 <= paved,
        "{phantom} of {paved} phantom road squares"
    );
    assert!(mills_seen > 0, "no mill drawn");
    assert!(far_mills.is_empty(), "{}", far_mills.join("\n"));
}
