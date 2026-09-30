//! Game-facing tactical scene data for arda (goals 48 and 65–69).
//!
//! [`build_scene`] turns a `TacticalLayout` plus its asset library into a
//! [`Scene`]: per-square SRD 5.1 movement, cover, obscurement and elevation;
//! merged wall polylines with door state; vision blockers; lights; terrain
//! regions and spawn hints. Because the painted image and the scene come from
//! the same layout, library and seed, they never disagree. An optional
//! [`RulesSidecar`] from the terrain generator overrides or merges
//! per-square rules.
//!
//! The queries ([`SceneIndex::line_of_sight`], [`SceneIndex::visible_squares`],
//! [`SceneIndex::path_cost`], [`SceneIndex::cover_between`]) exist for
//! server-side validation; the game may reimplement them from the JSON. The
//! JSON schema is documented in `crates/arda-scene/README.md`.

#![deny(unsafe_code)]
// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod debug;
pub mod error;
pub mod index;
pub mod los;
pub mod path;
pub mod regions;
pub mod rle;
pub mod serde_str;
pub mod sidecar;
pub mod spawn;
pub mod squares;
pub mod types;
pub mod walls;

pub use debug::scene_debug_png;
pub use error::SceneError;
pub use index::SceneIndex;
pub use rle::Grid;
pub use sidecar::{EdgeRole, EdgeRule, RulesCell, RulesExt, RulesSidecar, SIDECAR_FORMAT_VERSION};
pub use types::*;

use arda_tactical::{Library, TacticalLayout};

/// Builds the scene data for a layout.
///
/// `seed` must be the seed the image is rendered with: asset queries resolve
/// exactly as in `arda_tactical::render`, so the scene describes the props
/// the picture shows. `rules`, when given, must match the layout's size.
///
/// # Errors
/// Layout/library inconsistencies or a mismatched sidecar.
pub fn build_scene(
    layout: &TacticalLayout,
    lib: &Library,
    seed: u64,
    rules: Option<&RulesSidecar>,
) -> Result<Scene, SceneError> {
    layout.check(lib)?;
    // Keep build and parse symmetric: `from_json` refuses larger grids.
    if u64::from(layout.width) * u64::from(layout.height) > rle::MAX_SQUARES as u64 {
        return Err(SceneError::Schema(format!(
            "a {}x{} layout is larger than {} squares",
            layout.width,
            layout.height,
            rle::MAX_SQUARES
        )));
    }
    if let Some(r) = rules {
        r.check(layout.width, layout.height)?;
    }
    let placed = squares::resolve_all(layout, lib, seed)?;
    let layers = squares::derive(layout, &placed, rules);
    let regions = regions::build(&layers.movement.0, layout.width, layout.height);
    let mut scene = Scene {
        format_version: SCENE_FORMAT_VERSION,
        name: layout.name.clone(),
        width: layout.width,
        height: layout.height,
        seed,
        library: lib.catalog.library.clone(),
        library_version: lib.catalog.library_version.clone(),
        movement: layers.movement,
        climb: layers.climb,
        cover: layers.cover,
        obscured: layers.obscured,
        elevation_ft: layers.elevation_ft,
        water_depth_ft: layers.water_depth_ft,
        walls: walls::build(layout, lib, rules),
        vision_blockers: squares::vision_blockers(&placed),
        lights: spawn::lights(layout, &placed),
        regions,
        spawn_hints: SpawnHints::default(),
        tokens: Vec::new(),
    };
    scene.spawn_hints = spawn::hints(&scene.queries());
    Ok(scene)
}

impl Scene {
    /// Compact JSON: minified, per-square layers run-length encoded.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn to_json(&self) -> Result<String, SceneError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Pretty JSON, for reading.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn to_json_pretty(&self) -> Result<String, SceneError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parses a scene and checks every per-square layer has `width × height`
    /// squares.
    ///
    /// # Errors
    /// Malformed JSON, schema violations or layer size mismatches.
    pub fn from_json(json: &str) -> Result<Self, SceneError> {
        let s: Self = serde_json::from_str(json)?;
        if s.format_version != SCENE_FORMAT_VERSION {
            return Err(SceneError::Schema(format!(
                "format_version {} is not {SCENE_FORMAT_VERSION}",
                s.format_version
            )));
        }
        // Indexing allocates per edge, so an empty side with a huge other
        // side must not slip through the (vacuous) layer-length check.
        let n = u64::from(s.width) * u64::from(s.height);
        if s.width == 0 || s.height == 0 || n > rle::MAX_SQUARES as u64 {
            return Err(SceneError::Schema(format!(
                "a {}x{} scene is empty or larger than {} squares",
                s.width,
                s.height,
                rle::MAX_SQUARES
            )));
        }
        #[allow(clippy::cast_possible_truncation)] // n <= MAX_SQUARES
        let n = n as usize;
        for (i, wall) in s.walls.iter().enumerate() {
            walls::check_points(wall, s.width, s.height)
                .map_err(|e| SceneError::Schema(format!("wall {i}: {e}")))?;
        }
        let lens = [
            ("movement", s.movement.len()),
            ("climb", s.climb.len()),
            ("cover", s.cover.len()),
            ("obscured", s.obscured.len()),
            ("elevation_ft", s.elevation_ft.len()),
            ("water_depth_ft", s.water_depth_ft.len()),
        ];
        for (name, len) in lens {
            if len != n {
                return Err(SceneError::Schema(format!(
                    "layer `{name}` has {len} squares, expected {n}"
                )));
            }
        }
        Ok(s)
    }
}
