//! The political map (logic/16 §political-map): one distinct colour per
//! realm, hamlets only at area scale, and a legend that shows the styles
//! the map is drawn in.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    missing_docs
)]

use arda_settle::canvas::Canvas;
use arda_settle::model::Tier;
use arda_settle::render::layers::{
    hamlet_ids, road_lines, road_style, symbol_list, RoadStyle, CLASSES,
};
use arda_settle::render::legend::{self, BorderStyle, Key};
use arda_settle::render::palette::Palette;
use arda_settle::render::political::{band_width, BAND_MAX_PX, BAND_MIN_PX, LINE, LINE_PX};
use arda_settle::render::{Society, View};
use arda_settle::roads::RoadClass;
use arda_settle::{run, synthetic, PlaceParams};
use std::collections::{BTreeMap, BTreeSet};

fn society() -> Society {
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let grid = synthetic::landscape(params.seed).unwrap();
    let society = run(&grid, params).unwrap();
    let dir = std::env::temp_dir().join(format!("arda-settle-political-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    arda_settle::write(
        &dir.join(arda_settle::SOCIETY_DIR),
        &grid,
        params.seed,
        &society,
    )
    .unwrap();
    let soc = Society::read(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    soc
}

/// A `cols`-wide grid of realms, each bordering its four neighbours.
fn grid_graph(n: u64, cols: u64) -> BTreeMap<u64, Vec<u64>> {
    (1..=n)
        .map(|id| {
            let (x, y) = ((id - 1) % cols, (id - 1) / cols);
            let mut ns = Vec::new();
            for (dx, dy) in [(1_i64, 0_i64), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0 && ny >= 0 && (nx as u64) < cols {
                    let other = ny as u64 * cols + nx as u64 + 1;
                    if other <= n {
                        ns.push(other);
                    }
                }
            }
            (id, ns)
        })
        .collect()
}

fn channel_gap(a: [u8; 3], b: [u8; 3]) -> u32 {
    a.iter()
        .zip(&b)
        .map(|(x, y)| u32::from(x.abs_diff(*y)))
        .sum()
}

#[test]
fn no_two_realms_share_a_colour_and_neighbours_stand_apart() {
    for n in 1..=48_u64 {
        let graph = grid_graph(n, 4);
        let ids: Vec<u64> = graph.keys().copied().collect();
        let p = Palette::assign(&ids, &graph);
        let colours: BTreeSet<[u8; 3]> = ids.iter().map(|&id| p.colour(id)).collect();
        assert_eq!(colours.len(), ids.len(), "{n} realms share a colour");
        let hex: BTreeSet<String> = ids.iter().map(|&id| p.hex(id).unwrap()).collect();
        assert_eq!(hex.len(), ids.len());
        if n <= 16 {
            for (a, ns) in &graph {
                for b in ns {
                    let gap = channel_gap(p.colour(*a), p.colour(*b));
                    assert!(gap >= 90, "{n} realms: {a} and {b} too alike ({gap})");
                }
            }
        }
        // Deterministic.
        assert_eq!(p, Palette::assign(&ids, &graph));
    }
    let soc = society();
    let p = Palette::of(&soc.realms);
    let colours: BTreeSet<[u8; 3]> = soc.realms.realms.iter().map(|r| p.colour(r.id)).collect();
    assert_eq!(colours.len(), soc.realms.realms.len());
}

/// sRGB byte colour as OKLab (test-only: uses libm).
fn oklab(c: [u8; 3]) -> [f64; 3] {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(c[0]), lin(c[1]), lin(c[2]));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
    ]
}

fn delta_e(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (x, y) = (oklab(a), oklab(b));
    ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2) + (x[2] - y[2]).powi(2)).sqrt()
}

/// Smallest OKLab gap between any two of `cols`.
fn min_gap(cols: &[[u8; 3]]) -> f64 {
    let mut m = f64::INFINITY;
    for (i, a) in cols.iter().enumerate() {
        for b in &cols[i + 1..] {
            m = m.min(delta_e(*a, *b));
        }
    }
    m
}

/// The Atlas ground (olive lowland) a band must not vanish into.
const GROUND: [u8; 3] = [120, 130, 56];

#[test]
fn every_pair_of_realms_differs_perceptually_and_neighbours_more() {
    // 0.12 OKLab is about six just-noticeable differences (0.02): clearly
    // two colours at a glance, which is what a default-size world (12
    // realms) gets. The full sixteen keep 0.10; neighbours, which the
    // assignment pulls apart, keep 0.2.
    for n in 2..=16_u64 {
        let graph = grid_graph(n, 4);
        let ids: Vec<u64> = graph.keys().copied().collect();
        let p = Palette::assign(&ids, &graph);
        let cols: Vec<[u8; 3]> = ids.iter().map(|&id| p.colour(id)).collect();
        let floor = 0.12;
        let gap = min_gap(&cols);
        assert!(gap >= floor, "{n} realms: closest pair {gap:.3}");
        for (a, ns) in &graph {
            for b in ns {
                let d = delta_e(p.colour(*a), p.colour(*b));
                assert!(d >= 0.20, "{n} realms: neighbours {a},{b} only {d:.3}");
            }
        }
        for c in &cols {
            let g = delta_e(*c, GROUND);
            assert!(
                g >= 0.12,
                "{n} realms: a colour {c:?} sits on the ground ({g:.3})"
            );
        }
    }
    let soc = society();
    let p = Palette::of(&soc.realms);
    let cols: Vec<[u8; 3]> = soc.realms.realms.iter().map(|r| p.colour(r.id)).collect();
    assert!(min_gap(&cols) >= 0.12);
    for r in &soc.realms.realms {
        for nb in &r.neighbours {
            assert!(delta_e(p.colour(r.id), p.colour(*nb)) >= 0.20);
        }
    }
}

#[test]
fn the_band_is_a_visible_fraction_of_the_long_edge() {
    let at = |fw: f32, fh: f32| {
        band_width(&View {
            pw: fw,
            ph: fh,
            fw,
            fh,
            ox: 0.0,
            oy: 0.0,
            gw: 100,
            gh: 100,
            s: 1.0,
        })
    };
    // The default world is tall: 1940 x 4096.
    let b = at(1940.0, 4096.0);
    assert!(b >= 24.0, "{b} px on a 4096 px map");
    assert!(b / 4096.0 >= 0.005 && b / 4096.0 <= 0.008);
    assert!(
        at(4096.0, 1940.0) - b == 0.0,
        "the long edge, not the width"
    );
    // Sensible bounds at both extremes; whole-image size, not a tile's.
    assert!((at(256.0, 256.0) - BAND_MIN_PX).abs() < 1e-3);
    assert!((at(1e7, 1e7) - BAND_MAX_PX).abs() < 1e-3);
    assert!(at(8192.0, 8192.0) > at(4096.0, 4096.0));
}

fn view(soc: &Society, m_per_px: f32) -> View {
    let (gw, gh) = soc.size;
    let (fw, fh) = (gw as f32 * 100.0 / m_per_px, gh as f32 * 100.0 / m_per_px);
    View {
        pw: fw,
        ph: fh,
        fw,
        fh,
        ox: 0.0,
        oy: 0.0,
        gw,
        gh,
        s: 1.0,
    }
}

#[test]
fn hamlets_and_their_tracks_are_absent_at_world_scale_and_present_at_area_scale() {
    let soc = society();
    let seats: Vec<u64> = soc.realms.realms.iter().map(|r| r.seat).collect();
    let hamlets = |list: &[arda_settle::render::layers::Sym]| {
        list.iter().filter(|s| s.tier == Tier::Hamlet).count()
    };
    let world = view(&soc, 200.0);
    let area = view(&soc, 12.5);
    let at_world = symbol_list(
        &world,
        &soc.settlements,
        &seats,
        arda_settle::render::scale::shows_hamlets(world.m_per_px()),
    );
    let at_area = symbol_list(
        &area,
        &soc.settlements,
        &seats,
        arda_settle::render::scale::shows_hamlets(area.m_per_px()),
    );
    assert_eq!(hamlets(&at_world), 0, "no hamlet on a world map");
    assert!(
        hamlets(&at_area) > 0,
        "the fixture has hamlets on area maps"
    );
    assert_eq!(
        at_world.len(),
        at_area.len() - hamlets(&at_area),
        "cities, towns and villages all stay"
    );
    // Tracks are built to hamlets only, so none is left without one.
    let skip = hamlet_ids(&soc.settlements);
    let track = CLASSES.iter().position(|&c| c == RoadClass::Track).unwrap();
    assert!(!road_lines(&area, &soc.roads, &BTreeSet::new()).by_class[track].is_empty());
    assert!(road_lines(&world, &soc.roads, &skip).by_class[track].is_empty());
    let road = CLASSES.iter().position(|&c| c == RoadClass::Road).unwrap();
    assert_eq!(
        road_lines(&world, &soc.roads, &skip).by_class[road].len(),
        road_lines(&area, &soc.roads, &BTreeSet::new()).by_class[road].len(),
        "village roads stay"
    );
}

fn key(hamlets: bool) -> Key {
    Key {
        roads: RoadStyle::Ivory,
        borders: BorderStyle::EdgeBand,
        hamlets,
        realm_pair: [[200, 40, 40], [40, 80, 200]],
    }
}

#[test]
fn the_legend_lists_what_the_map_draws() {
    let world = key(false);
    assert!(legend::places(&world).iter().all(|p| p.0 != Tier::Hamlet));
    assert!(legend::roads(&world)
        .iter()
        .all(|r| r.0 != RoadClass::Track));
    assert!(legend::roads(&world)
        .iter()
        .any(|r| r.0 == RoadClass::Footpath));
    let area = key(true);
    assert!(legend::places(&area).iter().any(|p| p.0 == Tier::Hamlet));
    assert!(legend::roads(&area).iter().any(|r| r.0 == RoadClass::Track));
}

fn grey(w: usize, h: usize) -> Canvas {
    Canvas {
        width: w,
        height: h,
        rgb: vec![128; w * h * 3],
    }
}

fn px(c: &Canvas, x: usize, y: usize) -> [u8; 3] {
    let k = (y * c.width + x) * 3;
    [c.rgb[k], c.rgb[k + 1], c.rgb[k + 2]]
}

#[test]
fn legend_swatches_are_painted_in_the_map_styles() {
    let s = 2.0;
    // A highway swatch: ivory down the middle, the sepia casing beside it.
    let mut c = grey(80, 40);
    legend::road_swatch(
        &mut c,
        RoadStyle::Ivory,
        RoadClass::Highway,
        [(10.0, 20.0), (70.0, 20.0)],
        s,
    );
    let look = road_style(RoadStyle::Ivory, RoadClass::Highway);
    assert_eq!(px(&c, 40, 20), look.fill);
    let edge = (look.fill_w * s / 2.0 + (look.casing_w - look.fill_w) * s / 4.0) as usize;
    assert!(channel_gap(px(&c, 40, 20 + edge), look.casing) < 40);
    // A dotted footpath leaves gaps along its line.
    let mut c = grey(80, 40);
    legend::road_swatch(
        &mut c,
        RoadStyle::Ivory,
        RoadClass::Footpath,
        [(10.0, 20.0), (70.0, 20.0)],
        s,
    );
    let along: Vec<[u8; 3]> = (10..70).map(|x| px(&c, x, 20)).collect();
    let dot = road_style(RoadStyle::Ivory, RoadClass::Footpath).fill;
    assert!(
        along.iter().any(|p| channel_gap(*p, dot) < 40),
        "sepia dots"
    );
    assert!(
        along.iter().any(|p| channel_gap(*p, dot) > 150),
        "with gaps"
    );
    // The border: each realm's colour on its side, the dark dash-dot line
    // in the middle.
    let mut c = grey(80, 60);
    let pair = key(false).realm_pair;
    arda_settle::render::political::PoliticalLayer::swatch(
        &mut c,
        [(10.0, 30.0), (70.0, 30.0)],
        s,
        pair,
    );
    let above = px(&c, 40, 30 - (LINE_PX * s) as usize - 4);
    let below = px(&c, 40, 30 + (LINE_PX * s) as usize + 4);
    assert!(
        above[0] > above[2] && below[2] > below[0],
        "{above:?} {below:?}"
    );
    let line: Vec<[u8; 3]> = (10..70).map(|x| px(&c, x, 30)).collect();
    assert!(
        line.iter().any(|p| channel_gap(*p, LINE) < 30),
        "dash-dot line"
    );
    assert!(
        line.iter().any(|p| channel_gap(*p, LINE) > 120),
        "with gaps"
    );
}
