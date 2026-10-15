//! Composition does not depend on the window (review 3.5-8, goal 48;
//! logic/09 Invariant 3): on the real seed-42 MICRO world with every
//! society overlay, a cell composed on its own and the same cell cut from
//! a wider window that holds its borders inside agree on every square,
//! owner, rule, wall, edge rule, placement and light, including the walls
//! and edge rules on the cell's own border.
//!
//! World resolution as in `settlements.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    missing_docs
)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_blocks::society::SocietyOverlays;
use arda_blocks::{Composed, Pipeline};
use arda_tactical::layout::EdgeAxis;
use arda_tactical::Library;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

fn ready(dir: &Path) -> bool {
    dir.join("society/settlements.json").is_file()
        && dir.join("society/landuse.bin").is_file()
        && arda::World::load(dir)
            .is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == 42)
}

fn world_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = std::env::var_os("ARDA_TEST_WORLD") {
            return PathBuf::from(dir);
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = root.join("out/micro42");
        if ready(&out) {
            return out;
        }
        let dir = root.join("target/arda-people-fixture/micro42");
        fixture_dir::ensure(&dir, ready, |tmp| {
            arda::generate_from_fine_source(
                42,
                arda::GenerateConfig::MICRO,
                tmp,
                arda::FineDeliveryLimits::default(),
            )
            .expect("generating the MICRO fixture world");
            arda_settle::generate(tmp, arda_settle::grid::MEMORY_BUDGET).expect("settle");
        });
        dir
    })
    .clone()
}

fn lib() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

/// Everything of `c` inside the `64 × 64` region at `(ox, oy)`, in the
/// region's own coordinates, as comparable text.
#[derive(Debug, PartialEq, Eq)]
struct Region {
    squares: Vec<String>,
    walls: BTreeSet<String>,
    edges: BTreeSet<String>,
    placements: Vec<String>,
    lights: Vec<String>,
}

fn region(c: &Composed, (ox, oy): (u32, u32)) -> Region {
    let w = c.layout.width;
    let mut squares = Vec::new();
    for y in 0..64 {
        for x in 0..64 {
            let i = ((oy + y) * w + ox + x) as usize;
            squares.push(format!(
                "{x},{y} {:?} {:?} {}",
                c.layout.squares[i],
                c.owners[i],
                serde_json::to_string(&c.rules.squares[i]).unwrap()
            ));
        }
    }
    // An edge of the region: horizontal edges on rows 0..=64, vertical on
    // columns 0..=64, borders included.
    let inside = |x: u32, y: u32, axis: EdgeAxis| -> Option<(u32, u32)> {
        let (lx, ly) = (i64::from(x) - i64::from(ox), i64::from(y) - i64::from(oy));
        let ok = match axis {
            EdgeAxis::Horizontal => (0..64).contains(&lx) && (0..=64).contains(&ly),
            EdgeAxis::Vertical => (0..=64).contains(&lx) && (0..64).contains(&ly),
        };
        ok.then_some((lx as u32, ly as u32))
    };
    let walls = c
        .layout
        .walls
        .iter()
        .filter_map(|s| {
            let (x, y) = inside(s.x, s.y, s.axis)?;
            Some(format!(
                "{x},{y} {:?} {:?} {} {:?}",
                s.axis, s.kind, s.kit, s.tags
            ))
        })
        .collect();
    let edges = c
        .rules
        .edges
        .iter()
        .filter_map(|e| {
            let (x, y) = inside(e.x, e.y, e.axis)?;
            let mut e = e.clone();
            (e.x, e.y) = (x, y);
            Some(serde_json::to_string(&e).unwrap())
        })
        .collect();
    let within = |px: f32, py: f32| {
        let (lx, ly) = (px - ox as f32, py - oy as f32);
        ((0.0..64.0).contains(&lx) && (0.0..64.0).contains(&ly)).then_some((lx, ly))
    };
    let mut placements: Vec<String> = c
        .layout
        .placements
        .iter()
        .filter_map(|p| {
            let (x, y) = within(p.x, p.y)?;
            Some(format!("{x},{y} {:?} {} {}", p.asset, p.rotation, p.mirror))
        })
        .collect();
    placements.sort();
    let mut lights: Vec<String> = c
        .layout
        .lights
        .iter()
        .filter_map(|l| {
            let (x, y) = within(l.x, l.y)?;
            Some(format!("{x},{y} {} {:?}", l.radius_ft, l.colour))
        })
        .collect();
    lights.sort();
    Region {
        squares,
        walls,
        edges,
        placements,
        lights,
    }
}

/// Lines of `a` and `b` that differ, for the failure message.
fn differ<T: std::fmt::Debug + Ord + Clone>(
    what: &str,
    a: impl IntoIterator<Item = T>,
    b: impl IntoIterator<Item = T>,
) -> Vec<String> {
    let (a, b): (BTreeSet<T>, BTreeSet<T>) = (a.into_iter().collect(), b.into_iter().collect());
    a.symmetric_difference(&b)
        .take(4)
        .map(|d| format!("{what}: {d:?}"))
        .collect()
}

#[test]
fn a_cell_is_the_same_alone_and_inside_a_wider_window() {
    let world = Arc::new(arda_people::World::open(&world_dir()).unwrap());
    let society = SocietyOverlays::open(Arc::clone(&world)).unwrap();
    let pipeline = Pipeline::new(Box::new(world.src.clone()), 32);
    let library = lib();
    // The city and town cells of `settlements.rs`, and the contract kit's
    // town cell.
    let mut failures = Vec::new();
    for (gx, gy) in [(726_i64, 1687_i64), (400, 1449), (479, 1245)] {
        let alone = pipeline
            .window((gx * 64, gy * 64, 64, 64), &library, Some(&society))
            .unwrap();
        // A window whose interior holds the cell's four borders.
        let wide = pipeline
            .window(
                (gx * 64 - 24, gy * 64 - 24, 112, 112),
                &library,
                Some(&society),
            )
            .unwrap();
        let (a, b) = (region(&alone, (0, 0)), region(&wide, (24, 24)));
        let diffs: Vec<String> = [
            differ("square", a.squares.clone(), b.squares.clone()),
            differ("wall", a.walls.clone(), b.walls.clone()),
            differ("edge", a.edges.clone(), b.edges.clone()),
            differ("placement", a.placements.clone(), b.placements.clone()),
            differ("light", a.lights.clone(), b.lights.clone()),
        ]
        .concat();
        if !diffs.is_empty() {
            failures.push(format!("cell {gx},{gy}:\n  {}", diffs.join("\n  ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
