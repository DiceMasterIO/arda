//! Errors for mid-zoom refinement and relief tiles.

use thiserror::Error;

/// Everything that can stop a refinement or a relief tile.
#[derive(Debug, Error)]
pub enum MidzoomError {
    /// The world could not supply a layer.
    #[error("world layer unavailable: {0}")]
    Load(#[from] arda::LoadError),
    /// The fine terrain layer failed to read.
    #[error("fine terrain read failed: {0}")]
    Terrain(#[from] arda_core::TerrainFileError),
    /// Building an area's Atlas context failed.
    #[error("atlas context failed: {0}")]
    Export(#[from] arda::ExportError),
    /// Shading failed.
    #[error("relief shading failed: {0}")]
    Render(#[from] arda_render::RenderError),
    /// The tactical layer's water geometry failed to read.
    #[error("water geometry failed: {0}")]
    Water(#[from] arda_refine::RefineError),
    /// The world's society data (land use, roads, town plans) failed.
    #[error("society layer failed: {0}")]
    Society(String),
    /// The world has no recipe-5 fine terrain to refine.
    #[error("the world has no formed fine terrain layer")]
    NoFineTerrain,
    /// A window is empty, too large or has an unsupported spacing.
    #[error("invalid window: {0}")]
    Window(String),
    /// A tile lies outside the relief pyramid.
    #[error("tile {z}/{x}/{y} is outside the relief pyramid")]
    OutOfPyramid {
        /// Zoom.
        z: u32,
        /// Column.
        x: u32,
        /// Row.
        y: u32,
    },
    /// A shared cache lock was poisoned by a panicking thread.
    #[error("a cache lock was poisoned")]
    Poisoned,
    /// Memory for a window could not be reserved.
    #[error("allocation refused for {0}")]
    ResourceLimit(&'static str),
}
