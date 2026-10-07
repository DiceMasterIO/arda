//! Hand-built test layouts. WFC will generate layouts later; these exercise
//! every compositor feature against the placeholder library.

mod bench;
mod buildings;
mod farm;
mod riverside;
mod wild;

use crate::catalog::{AssetClass, WallRole};
use crate::layout::{AssetRef, EdgeAxis, LightSource, Placement, TacticalLayout, WallSegment};

/// Names of the built-in layouts.
pub const NAMES: [&str; 8] = [
    "riverside",
    "stone_warehouse",
    "timber_house",
    "wall_junctions",
    "forest_glade",
    "mountain_scree",
    "marsh",
    "farm_field",
];

pub use bench::benchmark;

/// A built-in layout by name. Besides [`NAMES`], `benchmark` gives the
/// 64 × 64 performance map.
#[must_use]
pub fn by_name(name: &str) -> Option<TacticalLayout> {
    match name {
        "riverside" => Some(riverside::build()),
        "stone_warehouse" => Some(buildings::stone_warehouse()),
        "timber_house" => Some(buildings::timber_house()),
        "wall_junctions" => Some(buildings::wall_junctions()),
        "forest_glade" => Some(wild::forest_glade()),
        "mountain_scree" => Some(wild::mountain_scree()),
        "marsh" => Some(wild::marsh()),
        "farm_field" => Some(farm::farm_field()),
        "benchmark" => Some(bench::benchmark(64)),
        _ => None,
    }
}

/// Every built-in layout.
#[must_use]
pub fn all() -> Vec<TacticalLayout> {
    NAMES.iter().filter_map(|n| by_name(n)).collect()
}

/// A small builder over [`TacticalLayout`] for readable hand-made maps.
/// Rectangles are half-open: `x0..x1`, `y0..y1` in squares.
struct Builder(TacticalLayout);

impl Builder {
    fn new(name: &str, w: u32, h: u32, ground: &str) -> Self {
        Self(TacticalLayout::new(name, w, h, ground))
    }

    fn ground(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, g: &str) -> &mut Self {
        self.ground_where(g, |x, y| x >= x0 && x < x1 && y >= y0 && y < y1)
    }

    fn ground_where(&mut self, g: &str, f: impl Fn(u32, u32) -> bool) -> &mut Self {
        for y in 0..self.0.height {
            for x in 0..self.0.width {
                if f(x, y) {
                    if let Some(sq) = self.0.square_mut(x, y) {
                        sq.ground = g.to_string();
                    }
                }
            }
        }
        self
    }

    fn water(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, depth: u8) -> &mut Self {
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some(sq) = self.0.square_mut(x, y) {
                    sq.water_depth_ft = depth;
                    sq.ground = "mud".into();
                }
            }
        }
        self
    }

    /// Water of `depth` over `ground` wherever `f(x, y)` holds.
    fn water_where(&mut self, depth: u8, ground: &str, f: impl Fn(u32, u32) -> bool) -> &mut Self {
        for y in 0..self.0.height {
            for x in 0..self.0.width {
                if f(x, y) {
                    if let Some(sq) = self.0.square_mut(x, y) {
                        sq.water_depth_ft = depth;
                        sq.ground = ground.to_string();
                    }
                }
            }
        }
        self
    }

    /// Sets every square's elevation from `f(x, y)`.
    fn elevation_by(&mut self, f: impl Fn(u32, u32) -> i16) -> &mut Self {
        for y in 0..self.0.height {
            for x in 0..self.0.width {
                if let Some(sq) = self.0.square_mut(x, y) {
                    sq.elevation_ft = f(x, y);
                }
            }
        }
        self
    }

    fn elevation(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, ft: i16) -> &mut Self {
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some(sq) = self.0.square_mut(x, y) {
                    sq.elevation_ft = ft;
                }
            }
        }
        self
    }

    fn edge(&mut self, x: u32, y: u32, axis: EdgeAxis, kind: WallRole, kit: &str) -> &mut Self {
        self.0.walls.push(WallSegment {
            x,
            y,
            axis,
            kind,
            kit: kit.to_string(),
            tags: Vec::new(),
        });
        self
    }

    /// Horizontal run along row line `y` from `x0` to `x1`.
    fn hrun(&mut self, x0: u32, x1: u32, y: u32, kit: &str) -> &mut Self {
        for x in x0..x1 {
            self.edge(x, y, EdgeAxis::Horizontal, WallRole::Run, kit);
        }
        self
    }

    /// Vertical run along column line `x` from `y0` to `y1`.
    fn vrun(&mut self, x: u32, y0: u32, y1: u32, kit: &str) -> &mut Self {
        for y in y0..y1 {
            self.edge(x, y, EdgeAxis::Vertical, WallRole::Run, kit);
        }
        self
    }

    /// The four walls around `x0..x1 × y0..y1`, with `floor` inside.
    fn room(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, kit: &str, floor: &str) -> &mut Self {
        self.ground(x0, y0, x1, y1, floor);
        self.hrun(x0, x1, y0, kit)
            .hrun(x0, x1, y1, kit)
            .vrun(x0, y0, y1, kit)
            .vrun(x1, y0, y1, kit)
    }

    /// Replaces the north edge of `(x, y)` with a door, window or gate.
    fn h(&mut self, x: u32, y: u32, kind: WallRole, kit: &str) -> &mut Self {
        self.edge(x, y, EdgeAxis::Horizontal, kind, kit)
    }

    /// Replaces the west edge of `(x, y)` with a door, window or gate.
    fn v(&mut self, x: u32, y: u32, kind: WallRole, kit: &str) -> &mut Self {
        self.edge(x, y, EdgeAxis::Vertical, kind, kit)
    }

    fn put(&mut self, id: &str, x: f32, y: f32, rotation: u16) -> &mut Self {
        self.0.placements.push(Placement {
            asset: AssetRef::Id(id.to_string()),
            x,
            y,
            rotation,
            mirror: false,
        });
        self
    }

    fn put_mirrored(&mut self, id: &str, x: f32, y: f32, rotation: u16) -> &mut Self {
        self.0.placements.push(Placement {
            asset: AssetRef::Id(id.to_string()),
            x,
            y,
            rotation,
            mirror: true,
        });
        self
    }

    fn query(&mut self, class: AssetClass, tag: &str, x: f32, y: f32) -> &mut Self {
        let asset = AssetRef::Query {
            class: Some(class),
            tags: vec![tag.to_string()],
        };
        self.0.placements.push(Placement {
            asset,
            x,
            y,
            rotation: 0,
            mirror: false,
        });
        self
    }

    fn light(&mut self, x: f32, y: f32, radius_ft: u16) -> &mut Self {
        self.0.lights.push(LightSource {
            x,
            y,
            radius_ft,
            colour: [255, 190, 110],
        });
        self
    }

    fn done(&mut self) -> TacticalLayout {
        std::mem::replace(&mut self.0, TacticalLayout::new("", 1, 1, "grass"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use std::path::Path;

    fn placeholder() -> Library {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    }

    #[test]
    fn every_layout_is_consistent_with_the_placeholders() {
        let lib = placeholder();
        assert!(all().len() >= 3);
        for l in all() {
            if let Err(e) = l.check(&lib) {
                panic!("{e}");
            }
        }
    }

    #[test]
    fn layouts_round_trip_through_json() {
        for l in all() {
            assert_eq!(TacticalLayout::from_json(&l.to_json().unwrap()).unwrap(), l);
        }
    }
}
