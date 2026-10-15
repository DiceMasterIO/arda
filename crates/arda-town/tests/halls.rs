//! Great buildings read as such inside (logic/10 §building-materials,
//! §town-wfc): civic and great buildings of masonry are partitioned by
//! masonry walls of their own kit, and a manor's great hall is furnished
//! by its size. The city's libraries are checked on the seed-42 world
//! (`arda-people/tests/interiors.rs`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_wrap,
    missing_docs
)]

mod common;

use arda_tactical::layout::EdgeAxis;
use arda_town::block::wfc::{indoor, review};
use arda_town::block::{self, kits, Window};
use arda_town::BuildingFunction as F;
use common::plans;

fn great(f: F) -> bool {
    matches!(f, F::Manor | F::Library | F::Keep | F::Temple | F::School)
}

#[test]
fn great_masonry_buildings_are_partitioned_in_masonry() {
    let mut seen = 0;
    for pl in plans() {
        for b in pl.buildings.iter().filter(|b| great(b.function)) {
            let look = kits::look(pl, b);
            if !look.material.is_stone() {
                continue;
            }
            assert_eq!(
                look.partition, look.shell,
                "{} {:?} {}",
                pl.name, b.function, b.id.0
            );
            // The drawn partitions carry that kit.
            let r = b.rect;
            let win = Window {
                x: r.x0 - 1,
                y: r.y0 - 1,
                w: r.x1 - r.x0 + 2,
                h: r.y1 - r.y0 + 2,
            };
            let blk = block::generate(pl, win).unwrap();
            for w in blk.layout.walls.iter().filter(|w| w.is_partition()) {
                let (gx, gy) = (win.x + i64::from(w.x), win.y + i64::from(w.y));
                let inside = match w.axis {
                    EdgeAxis::Horizontal => r.contains(gx, gy) && gy > r.y0,
                    EdgeAxis::Vertical => r.contains(gx, gy) && gx > r.x0,
                };
                if inside {
                    assert_eq!(w.kit, look.shell, "{} {:?} {}", pl.name, b.function, b.id.0);
                    seen += 1;
                }
            }
        }
    }
    assert!(seen > 0, "no masonry partition drawn");
}

#[test]
fn manor_great_halls_are_furnished_by_their_size() {
    let mut halls = 0;
    for pl in plans() {
        for b in pl.buildings.iter().filter(|b| b.function == F::Manor) {
            let Ok(Some(p)) = indoor::solve(pl, b) else {
                continue;
            };
            for r in review::rooms(&p)
                .into_iter()
                .filter(|r| r.tag == "great_hall")
            {
                if r.area < 48 {
                    continue;
                }
                let id = b.id.0;
                let tables = r.count("prop.table");
                assert!(
                    tables as i64 >= r.area / 18,
                    "manor {id}: {tables} tables, {} sq",
                    r.area
                );
                assert_eq!(r.count("prop.hearth"), 1, "manor {id}");
                assert!(
                    r.count("prop.banner") as i64 >= r.area / 30,
                    "manor {id}: banners"
                );
                assert!(r.count("rug") >= 1, "manor {id}: rug");
                halls += 1;
            }
        }
    }
    assert!(halls > 0, "no large great hall");
}
