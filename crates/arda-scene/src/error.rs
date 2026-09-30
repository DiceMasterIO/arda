//! Errors for the scene crate.

use thiserror::Error;

/// Anything that can go wrong building, querying or drawing a scene.
#[derive(Debug, Error)]
pub enum SceneError {
    /// The layout is inconsistent with itself or with the library.
    #[error(transparent)]
    Tactical(#[from] arda_tactical::TacticalError),
    /// The rules sidecar does not match the layout.
    #[error("rules sidecar: {0}")]
    Sidecar(String),
    /// JSON could not be parsed or written.
    #[error("scene json: {0}")]
    Json(#[from] serde_json::Error),
    /// A square lies outside the scene.
    #[error("square ({x}, {y}) is outside the {width}x{height} scene")]
    OutOfBounds {
        /// Column.
        x: u32,
        /// Row.
        y: u32,
        /// Scene width.
        width: u32,
        /// Scene height.
        height: u32,
    },
    /// A wall index does not exist or names a wall that cannot open.
    #[error("wall {0}: {1}")]
    Wall(usize, String),
    /// A scene document breaks the schema beyond what serde checks.
    #[error("scene: {0}")]
    Schema(String),
    /// PNG encoding failed.
    #[error("png: {0}")]
    Png(String),
    /// A debug-render option is out of range.
    #[error("debug render: {0}")]
    Options(String),
}
