//! Stage rules on the synthetic landscape (spec "Tests").
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::crossings::CrossingKind;
use arda_settle::grid::Grid;
use arda_settle::model::Tier;
use arda_settle::num::dist_m;
use arda_settle::place::TOWN_SPACING_M;
use arda_settle::{run, synthetic, PlaceParams, Society};
use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const PARAMS: PlaceParams = PlaceParams {
    seed: 42,
    density_per_km2: 15,
};

struct Fixture {
    grid: Grid,
    society: Society,
    elapsed: Duration,
}

static FIXTURE: LazyLock<Fixture> = LazyLock::new(|| {
    let grid = synthetic::landscape(PARAMS.seed).unwrap();
    let start = Instant::now();
    let society = run(&grid, PARAMS).unwrap();
    Fixture {
        grid,
        society,
        elapsed: start.elapsed(),
    }
});

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("arda-settle-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn same_seed_gives_byte_identical_output() {
    let f = &*FIXTURE;
    let again = run(&f.grid, PARAMS).unwrap();
    let (a, b) = (TempDir::new("det-a"), TempDir::new("det-b"));
    arda_settle::write(&a.0, &f.grid, PARAMS.seed, &f.society).unwrap();
    arda_settle::write(&b.0, &f.grid, PARAMS.seed, &again).unwrap();
    let (fa, fb) = (files(&a.0), files(&b.0));
    let names: Vec<&str> = fa.iter().map(|(n, _)| n.as_str()).collect();
    for want in [
        "landuse.bin",
        "names.json",
        "realms.bin",
        "realms.json",
        "roads.json",
        "settlements.json",
        "stats.json",
    ] {
        assert!(names.contains(&want), "missing {want}");
    }
    assert_eq!(fa, fb);
}

#[test]
fn settlements_are_refused_from_steep_wet_and_flooded_ground() {
    let f = &*FIXTURE;
    let g = &f.grid;
    assert!(f.society.settlements.len() > 50);
    for s in &f.society.settlements {
        let i = s.index(g.width);
        assert!(g.is_land(i), "{} on water", s.name);
        assert!(!g.is_watercourse(i), "{} in a channel", s.name);
        assert!(g.slope_md[i] <= 14_000, "{} on {}°", s.name, g.slope_md[i]);
        assert!(
            g.har_dm[i] >= 15,
            "{} only {} dm above the river",
            s.name,
            g.har_dm[i]
        );
    }
}

#[test]
fn towns_keep_their_distance_and_follow_rank_size() {
    let f = &*FIXTURE;
    let towns: Vec<_> = f
        .society
        .settlements
        .iter()
        .filter(|s| s.tier.is_urban())
        .collect();
    assert!(towns.len() >= 3);
    for (k, a) in towns.iter().enumerate() {
        for b in &towns[k + 1..] {
            let d = dist_m(
                i64::from(a.cell_x) - i64::from(b.cell_x),
                i64::from(a.cell_y) - i64::from(b.cell_y),
            );
            assert!(
                d >= TOWN_SPACING_M,
                "{} and {} are {d} m apart",
                a.name,
                b.name
            );
        }
    }
    let st = &f.society.stats;
    assert!(
        (-1.1..=-0.9).contains(&st.rank_size_slope),
        "{}",
        st.rank_size_slope
    );
    // Primate capitals (one city per realm, `primacy.rs`) bend the top of
    // the list, so the fit is looser than a pure rank-size list's.
    assert!(st.rank_size_r2 > 0.8, "{}", st.rank_size_r2);
    let hamlets = st.count_by_tier["hamlet"];
    let villages = st.count_by_tier["village"];
    assert!(hamlets > villages && villages > towns.len() as u64);
}

#[test]
fn towns_are_central_places_inland_and_regularly_spaced() {
    let st = &FIXTURE.society.stats;
    eprintln!(
        "inland {:.2}, nni {:.2}, nn {:.1} km",
        st.inland_town_share, st.town_nni, st.town_nn_km
    );
    assert!(
        (0.3..=0.85).contains(&st.inland_town_share),
        "inland share {}",
        st.inland_town_share
    );
    assert!(st.town_nni > 1.3, "nearest-neighbour index {}", st.town_nni);
}

#[test]
fn farmland_is_about_point_eight_hectares_a_head() {
    let st = &FIXTURE.society.stats;
    assert!(
        (0.7..=0.9).contains(&st.farmland_ha_per_person),
        "{}",
        st.farmland_ha_per_person
    );
    let placed = st.population as f64 / st.target_population as f64;
    assert!((0.95..=1.05).contains(&placed), "{placed}");
}

#[test]
fn every_settlement_is_on_one_connected_network() {
    let f = &*FIXTURE;
    let g = &f.grid;
    let raster = &f.society.network.raster;
    let start = f.society.settlements[0].index(g.width);
    let mut seen = vec![false; g.len()];
    let mut q = VecDeque::from([start]);
    seen[start] = true;
    while let Some(c) = q.pop_front() {
        for (j, _, _) in g.neighbours8(c) {
            if !seen[j] && raster[j] != 0 {
                seen[j] = true;
                q.push_back(j);
            }
        }
    }
    assert!(f.society.network.unreachable.is_empty());
    for s in &f.society.settlements {
        assert!(seen[s.index(g.width)], "{} is cut off", s.name);
    }
}

#[test]
fn water_is_only_crossed_at_recorded_crossings() {
    let f = &*FIXTURE;
    let g = &f.grid;
    let raster = &f.society.network.raster;
    for c in &f.society.crossings {
        let i = g.at(c.x_m / 100, c.y_m / 100).unwrap();
        assert!(raster[i] != 0, "crossing {} off the road", c.id);
        match c.water.as_str() {
            "river" => {
                assert!(g.is_watercourse(i), "crossing {} not on a river", c.id);
                assert!(c.width_m == g.width_m(i));
            }
            _ => assert_eq!(c.kind, CrossingKind::Ferry),
        }
    }
    // Every road cell on open water or a channel belongs to a crossing
    // component that holds a recorded crossing.
    let at: BTreeSet<usize> = f
        .society
        .crossings
        .iter()
        .filter_map(|c| g.at(c.x_m / 100, c.y_m / 100))
        .collect();
    let mut seen = vec![false; g.len()];
    for i in 0..g.len() {
        let wet = |j: usize| raster[j] != 0 && (g.is_water(j) || g.is_watercourse(j));
        if seen[i] || !wet(i) {
            continue;
        }
        let mut stack = vec![i];
        seen[i] = true;
        let mut found = false;
        while let Some(c) = stack.pop() {
            found |= at.contains(&c);
            for (j, _, _) in g.neighbours8(c) {
                if !seen[j] && wet(j) {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        assert!(found, "unrecorded crossing near cell {i}");
    }
}

#[test]
fn open_ground_sinuosity_is_between_1_2_and_1_4() {
    let st = &FIXTURE.society.stats;
    assert!(st.open_routes >= 20);
    assert!(
        (1.2..=1.4).contains(&st.sinuosity_open),
        "{}",
        st.sinuosity_open
    );
    let km = &st.road_km;
    assert!(km["highway"] > 0.0 && km["road"] > 0.0 && km["track"] > 0.0);
}

#[test]
fn mountain_roads_wind_without_runaway_detours() {
    let st = &FIXTURE.society.stats;
    let by = &st.sinuosity_by_terrain;
    eprintln!("{by:?}, runaways {}", st.runaway_routes);
    let routes: u64 = by.values().map(|s| s.routes).sum();
    assert!(routes >= 20 && by.contains_key("mountain") && by.contains_key("hill"));
    for (k, s) in by {
        if s.routes >= 5 && k != "open" {
            assert!(
                (1.2..=2.0).contains(&s.median),
                "{k} median sinuosity {}",
                s.median
            );
        }
    }
    assert!(
        st.runaway_routes * 20 <= routes,
        "{} of {routes} routes are runaway detours",
        st.runaway_routes
    );
}

#[test]
fn realms_partition_the_land_with_closed_borders() {
    let f = &*FIXTURE;
    let g = &f.grid;
    let realms = &f.society.realms;
    let n = u16::try_from(realms.realms.len()).unwrap();
    assert!(n >= 2, "two or more towns must give two or more realms");
    for i in 0..g.len() {
        let r = realms.map[i];
        if g.is_land(i) {
            assert!((1..=n).contains(&r), "land cell {i} has realm {r}");
        } else {
            assert_eq!(r, 0);
        }
    }
    for s in &f.society.settlements {
        assert_eq!(u64::from(realms.map[s.index(g.width)]), s.realm_id.get());
    }
    // Borders are closed: inside the land every corner where a realm's
    // border polylines end is shared by an even number of ends, so the
    // lines join up; only corners on water or the map edge may end a line.
    let touches_edge = |x: i64, y: i64| {
        [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)]
            .iter()
            .any(|&(cx, cy)| g.at(cx, cy).is_none_or(|j| !g.is_land(j)))
    };
    for r in &realms.realms {
        assert!(!r.name.is_empty());
        let mut ends: std::collections::BTreeMap<[i64; 2], u32> = std::collections::BTreeMap::new();
        for line in &r.borders {
            let (a, b) = (line[0], line[line.len() - 1]);
            if a != b {
                *ends.entry(a).or_default() += 1;
                *ends.entry(b).or_default() += 1;
            }
        }
        for (p, n) in ends {
            assert!(
                n % 2 == 0 || touches_edge(p[0] / 100, p[1] / 100),
                "realm {} border dangles at {p:?}",
                r.id
            );
        }
    }
    let cities: usize = f
        .society
        .settlements
        .iter()
        .filter(|s| s.tier == Tier::City)
        .count();
    assert!(cities <= 2 * realms.realms.len());
}

#[test]
fn borders_snap_to_rivers_and_ridges_and_realms_stay_whole() {
    let f = &*FIXTURE;
    let g = &f.grid;
    let st = &f.society.stats;
    eprintln!(
        "border on line {} (raw {}), of all {}",
        st.border_on_river_or_ridge_pm,
        st.border_on_river_or_ridge_raw_pm,
        st.border_natural_of_all_pm
    );
    assert!(
        st.border_on_river_or_ridge_pm >= 900,
        "only {} per mille of snappable border is on a line",
        st.border_on_river_or_ridge_pm
    );
    assert!(st.border_on_river_or_ridge_pm >= st.border_on_river_or_ridge_raw_pm);
    // Every 8-connected fragment of a realm either holds its seat or
    // touches no other realm (it is alone on its island).
    let map = &f.society.realms.map;
    let seat_cell = |r: u16| {
        let seat = f.society.realms.realms[usize::from(r) - 1].seat;
        f.society
            .settlements
            .iter()
            .find(|s| s.id.get() == seat)
            .unwrap()
            .index(g.width)
    };
    let mut seen = vec![false; g.len()];
    for s in 0..g.len() {
        if map[s] == 0 || seen[s] {
            continue;
        }
        let mut stack = vec![s];
        seen[s] = true;
        let (mut has_seat, mut touches) = (false, false);
        let seat = seat_cell(map[s]);
        while let Some(a) = stack.pop() {
            has_seat |= a == seat;
            for (b, _, _) in g.neighbours8(a) {
                if map[b] == map[a] && !seen[b] {
                    seen[b] = true;
                    stack.push(b);
                } else if map[b] != 0 && map[b] != map[a] {
                    touches = true;
                }
            }
        }
        assert!(has_seat || !touches, "realm {} has an exclave", map[s]);
    }
}

#[test]
fn names_are_unique_within_their_scope() {
    let f = &*FIXTURE;
    let unique = |v: Vec<&str>| {
        let n = v.len();
        let set: BTreeSet<&str> = v.into_iter().collect();
        assert_eq!(set.len(), n);
    };
    unique(
        f.society
            .settlements
            .iter()
            .map(|s| s.name.as_str())
            .collect(),
    );
    unique(
        f.society
            .realms
            .realms
            .iter()
            .map(|r| r.name.as_str())
            .collect(),
    );
    unique(f.society.rivers.iter().map(|r| r.name.as_str()).collect());
    unique(
        f.society
            .mountains
            .iter()
            .map(|m| m.name.as_str())
            .collect(),
    );
    assert!(f
        .society
        .settlements
        .iter()
        .all(|s| !s.name.is_empty() && !s.history.is_empty()));
    assert!(!f.society.rivers.is_empty());
}

#[test]
fn settlement_records_match_the_npc_profile_contract() {
    let f = &*FIXTURE;
    let json = serde_json::to_value(&f.society.settlements[0]).unwrap();
    for key in [
        "id",
        "name",
        "tier",
        "population",
        "functions",
        "wealth",
        "culture",
        "realm_id",
        "biome",
        "coastal",
        "riverine",
    ] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    // Ids and seeds are JSON strings (convention I5, I17).
    assert!(json["id"].is_string() && json["realm_id"].is_string());
    let road = serde_json::to_value(&f.society.network.roads[0]).unwrap();
    assert!(road["id"].is_string() && road["from"].is_string());
    let biomes = [
        "temperate",
        "warm_temperate",
        "temperate_forest",
        "boreal_forest",
        "highland",
        "alpine",
        "wetland",
        "steppe",
        "coastal",
    ];
    assert!(biomes.contains(&json["biome"].as_str().unwrap()));
    let tiers = ["hamlet", "village", "town", "city"];
    let cultures = [
        "heartland",
        "highland",
        "sylvan",
        "coastal",
        "southern",
        "borderland",
    ];
    for s in &f.society.settlements {
        let v = serde_json::to_value(s).unwrap();
        assert!(tiers.contains(&v["tier"].as_str().unwrap()));
        assert!(cultures.contains(&s.culture.as_str()));
        assert!(!s.functions.is_empty());
        assert!(s.realm_id.get() >= 1);
    }
}

#[test]
fn mining_is_rare_and_clustered() {
    let st = &FIXTURE.society.stats;
    assert!(
        st.mining_share_pm <= 60,
        "{} per mille mine",
        st.mining_share_pm
    );
    if st.mining_settlements >= 2 {
        assert!(
            st.mining_clustered_pm >= 700,
            "only {} per mille of mines have a neighbour",
            st.mining_clustered_pm
        );
    }
}

#[test]
fn a_small_world_runs_in_seconds() {
    assert!(
        FIXTURE.elapsed < Duration::from_secs(30),
        "{:?}",
        FIXTURE.elapsed
    );
}

/// The settlement stage's records must load, unchanged, as arda-society's
/// input: every id form, road record and biome key (the settle → society
/// contract, vocabulary I5 and I22).
#[test]
fn society_reads_the_records_this_stage_writes() {
    let s = &FIXTURE.society;
    let doc = serde_json::json!({
        "settlements": serde_json::to_value(&s.settlements).unwrap(),
        "roads": serde_json::to_value(&s.network.roads).unwrap(),
        "realms": serde_json::to_value(&s.realms.realms).unwrap(),
    });
    let world: arda_society::WorldSettlements = serde_json::from_value(doc).unwrap();
    assert_eq!(world.roads.len(), s.network.roads.len());
    assert!(!world.roads.is_empty());
    let society = arda_society::simulate_society(PARAMS.seed, &world).unwrap();
    assert_eq!(society.settlements.len(), s.settlements.len());
}

/// With no towns the first village stands in as the hub; it has nowhere to
/// connect to and must not be reported as cut off.
#[test]
fn the_stand_in_hub_is_not_reported_unreachable() {
    let grid = synthetic::landscape(7).unwrap();
    let society = run(
        &grid,
        PlaceParams {
            seed: 7,
            density_per_km2: 1,
        },
    )
    .unwrap();
    let urban = society.settlements.iter().any(|s| s.tier.is_urban());
    assert!(!urban, "the fixture needs a world without towns");
    let hub = society
        .settlements
        .iter()
        .find(|s| s.tier == Tier::Village)
        .expect("a village");
    assert!(
        !society.network.unreachable.contains(&hub.id.get()),
        "{:?}",
        society.network.unreachable
    );
}

#[test]
fn raster_headers_cannot_claim_unbounded_sizes() {
    // A 21-byte file claiming 65535 x 65535 cells (21 GB decoded) must be
    // refused from its header, before any allocation.
    let dir = TempDir::new("bomb");
    std::fs::create_dir_all(&dir.0).unwrap();
    let path = dir.0.join("landuse.bin");
    let mut bytes = b"ARDALND\0".to_vec();
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&65535u32.to_le_bytes());
    bytes.extend_from_slice(&65535u32.to_le_bytes());
    bytes.push(0);
    std::fs::write(&path, &bytes).unwrap();
    let err = arda_settle::output::read_landuse(&path).unwrap_err();
    assert!(
        matches!(err, arda_settle::error::SettleError::Format { .. }),
        "{err:?}"
    );
    let empty = [
        b"ARDALND\0".as_slice(),
        &1u32.to_le_bytes(),
        &0u32.to_le_bytes(),
        &5u32.to_le_bytes(),
    ]
    .concat();
    std::fs::write(&path, &empty).unwrap();
    assert!(arda_settle::output::read_landuse(&path).is_err());
}
