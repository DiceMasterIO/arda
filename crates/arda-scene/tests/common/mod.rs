//! Shared helpers for the scene integration tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_scene::{build_scene, RulesSidecar, Scene};
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::{AssetRef, EdgeAxis, LightSource, Placement, WallSegment};
use arda_tactical::{Library, TacticalLayout};
use std::path::Path;
use std::sync::OnceLock;

pub fn lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    })
}

pub struct L(pub TacticalLayout);

impl L {
    pub fn new(w: u32, h: u32) -> Self {
        Self(TacticalLayout::new("test", w, h, "grass"))
    }

    pub fn edge(mut self, axis: EdgeAxis, x: u32, y: u32, kind: WallRole) -> Self {
        self.0.walls.push(WallSegment {
            x,
            y,
            axis,
            kind,
            kit: "stone".into(),
        });
        self
    }

    /// Wall on the west edge of `(x, y)`.
    pub fn v(self, x: u32, y: u32, kind: WallRole) -> Self {
        self.edge(EdgeAxis::Vertical, x, y, kind)
    }

    /// Wall on the north edge of `(x, y)`.
    pub fn h(self, x: u32, y: u32, kind: WallRole) -> Self {
        self.edge(EdgeAxis::Horizontal, x, y, kind)
    }

    /// An asset centred on square `(x, y)`.
    pub fn put(mut self, id: &str, x: u32, y: u32) -> Self {
        self.0.placements.push(Placement {
            asset: AssetRef::Id(id.into()),
            x: x as f32 + 0.5,
            y: y as f32 + 0.5,
            rotation: 0,
            mirror: false,
        });
        self
    }

    pub fn light(mut self, x: f32, y: f32, radius_ft: u16) -> Self {
        self.0.lights.push(LightSource {
            x,
            y,
            radius_ft,
            colour: [255, 200, 120],
        });
        self
    }

    pub fn water(mut self, x: u32, y: u32, depth: u8) -> Self {
        self.0.square_mut(x, y).unwrap().water_depth_ft = depth;
        self
    }

    pub fn elevation(mut self, x: u32, y: u32, ft: i16) -> Self {
        self.0.square_mut(x, y).unwrap().elevation_ft = ft;
        self
    }

    pub fn scene(&self) -> Scene {
        build_scene(&self.0, lib(), 1, None).unwrap()
    }

    pub fn scene_with(&self, rules: &RulesSidecar) -> Scene {
        build_scene(&self.0, lib(), 1, Some(rules)).unwrap()
    }
}
