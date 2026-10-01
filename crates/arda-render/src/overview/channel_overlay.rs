//! Canonical saved-channel coverage for the opt-in Atlas overview.
use crate::RenderError;
use arda_core::{
    hydrology::{ChannelEdge, ReachId},
    AreaCoord, AreaObjects, GlobalCell,
};
use std::collections::BTreeSet;

mod shapes;
mod tile;
pub(super) use tile::ChannelTile;

const LEGACY_DISPLAY_WIDTH_DM: u64 = 10_000;
// Recipe 4 uses a narrower symbol to keep major channels legible without
// covering their carved valleys at fitted overview scale.
const FINE_DISPLAY_WIDTH_DM: u64 = 4_000;
// Recipe 5: four times physical width, so trunk rivers read at fitted-world
// scale and small streams appear as the export grows (goals 28, 33).
const FORMED_DISPLAY_WIDTH_DM: u64 = 12_000;
const CONTEXT_CELLS: i64 = 7;
const MAX_CONTEXT_EDGES: usize = 262_144;
const MAX_SHAPES: usize = 1_048_576;
const MAX_VERTICES: usize = 4_194_304;
const MAX_PIXEL_CAPACITY: usize = 1_048_576;
const MAX_PIXEL_CANDIDATES: usize = 4096;
const MAX_WORK: u64 = 250_000_000;
fn bad(reason: &'static str) -> RenderError {
    RenderError::ChannelGeometry { reason }
}
fn context_error(reason: &'static str) -> RenderError {
    RenderError::AtlasContext { reason }
}
fn relevant(at: AreaCoord, edge: ChannelEdge) -> bool {
    let x = i64::from(at.x) * 512;
    let y = i64::from(at.y) * 512;
    let minx = i64::from(edge.from.x.min(edge.to.x));
    let maxx = i64::from(edge.from.x.max(edge.to.x));
    let miny = i64::from(edge.from.y.min(edge.to.y));
    let maxy = i64::from(edge.from.y.max(edge.to.y));
    minx <= x + 511 + CONTEXT_CELLS
        && maxx >= x - CONTEXT_CELLS
        && miny <= y + 511 + CONTEXT_CELLS
        && maxy >= y - CONTEXT_CELLS
}
fn key(e: &ChannelEdge) -> (GlobalCell, GlobalCell) {
    (e.from, e.to)
}
/// Canonical, bounded saved geometry for one Atlas overview area, including its adjacent-area context.
pub struct OverviewChannelContext {
    target: AreaCoord,
    areas_wide: i32,
    areas_high: i32,
    seen: BTreeSet<AreaCoord>,
    edges: Vec<ChannelEdge>,
    /// Braided course cells with their belt width, dm (logic/04
    /// §atlas-formed braids).
    braided: std::collections::BTreeMap<GlobalCell, u32>,
    ready: bool,
}
impl OverviewChannelContext {
    /// Starts a context for one area of the saved exported rectangle.
    pub fn new(target: AreaCoord, areas_wide: i32, areas_high: i32) -> Result<Self, RenderError> {
        if !(1..=78).contains(&areas_wide)
            || !(1..=78).contains(&areas_high)
            || target.x < 0
            || target.y < 0
            || target.x >= areas_wide
            || target.y >= areas_high
        {
            return Err(context_error("invalid Atlas channel target rectangle"));
        }
        Ok(Self {
            target,
            areas_wide,
            areas_high,
            seen: BTreeSet::new(),
            edges: Vec::new(),
            braided: std::collections::BTreeMap::new(),
            ready: false,
        })
    }
    /// Adds one saved area's bounded channel edges and terminal reach points.
    pub fn add_area(&mut self, at: AreaCoord, objects: &AreaObjects) -> Result<(), RenderError> {
        if self.ready {
            return Err(context_error("Atlas channel context already finished"));
        }
        if at.x < 0
            || at.y < 0
            || at.x >= self.areas_wide
            || at.y >= self.areas_high
            || at.x.abs_diff(self.target.x) > 1
            || at.y.abs_diff(self.target.y) > 1
        {
            return Err(context_error(
                "Atlas channel context area outside adjacent neighborhood",
            ));
        }
        if !self.seen.insert(at) {
            return Err(context_error("duplicate Atlas channel context area"));
        }
        for &edge in &objects.channel_edges {
            if relevant(self.target, edge) {
                self.push(edge)?;
            }
        }
        for reach in &objects.global.reaches {
            if !reach.id.is_point() || reach.mean_discharge.raw() < 40 {
                continue;
            }
            if reach.from != reach.to
                || reach.id
                    != ReachId::point(reach.from)
                        .ok_or_else(|| bad("invalid saved terminal identity"))?
            {
                return Err(bad("invalid saved terminal point"));
            }
            let width = arda_core::hydrology::channel_width_dm(reach.mean_discharge)
                .ok_or_else(|| bad("terminal width overflow"))?;
            let edge = ChannelEdge {
                from: reach.from,
                to: reach.to,
                from_width_dm: width,
                to_width_dm: width,
                discharge: reach.mean_discharge,
            };
            if relevant(self.target, edge) {
                self.push(edge)?;
            }
        }
        Ok(())
    }
    /// Marks saved braided course cells (global) with their belt width in
    /// decimetres; recipe-5 overviews draw two extra threads weaving
    /// across the belt there.
    pub fn add_braided(
        &mut self,
        cells: impl IntoIterator<Item = (GlobalCell, u32)>,
    ) -> Result<(), RenderError> {
        if self.ready {
            return Err(context_error("Atlas channel context already finished"));
        }
        for (cell, belt) in cells {
            let e = self.braided.entry(cell).or_insert(belt);
            *e = (*e).max(belt);
        }
        Ok(())
    }
    fn push(&mut self, edge: ChannelEdge) -> Result<(), RenderError> {
        if self.edges.len() == MAX_CONTEXT_EDGES {
            return Err(bad("Atlas overview context exceeds 262144 channel edges"));
        }
        self.edges.push(edge);
        Ok(())
    }
    /// Requires every existing adjacent saved area and canonicalizes duplicate halo records.
    pub fn finish(mut self) -> Result<Self, RenderError> {
        for y in (self.target.y - 1).max(0)..=(self.target.y + 1).min(self.areas_high - 1) {
            for x in (self.target.x - 1).max(0)..=(self.target.x + 1).min(self.areas_wide - 1) {
                if !self.seen.contains(&AreaCoord::new(x, y)) {
                    return Err(context_error("missing adjacent Atlas channel context"));
                }
            }
        }
        self.edges.sort_unstable_by_key(|e| {
            (
                e.from,
                e.to,
                e.from_width_dm,
                e.to_width_dm,
                e.discharge.raw(),
            )
        });
        self.edges.dedup();
        if self.edges.windows(2).any(|p| key(&p[0]) == key(&p[1])) {
            return Err(bad(
                "duplicate channel endpoints disagree about saved fields",
            ));
        }
        self.ready = true;
        Ok(self)
    }
    fn edges(&self, at: AreaCoord) -> Result<&[ChannelEdge], RenderError> {
        if !self.ready || self.target != at {
            return Err(context_error(
                "Atlas channel context target or completeness mismatch",
            ));
        }
        Ok(&self.edges)
    }
}
/// Recipe-5 river water as v0.1 drew it.
const fn formed_river_colour_v5(band: super::RiverBand) -> [u8; 3] {
    match band {
        super::RiverBand::Light => [70, 150, 172],
        super::RiverBand::Mid => [44, 128, 164],
        super::RiverBand::Dark => [28, 100, 150],
    }
}

/// Recipe-5 minimum on-screen width by discharge, pixels Q8 (v0.1 steps).
const fn min_width_px_q8_v5(discharge: u64) -> i64 {
    if discharge >= 50_000 {
        410
    } else if discharge >= 5_000 {
        256
    } else {
        0
    }
}

/// Blue-teal formed river water, deepening downstream (goal 28; recipe 6).
pub(crate) const fn formed_river_colour(band: super::RiverBand) -> [u8; 3] {
    match band {
        super::RiverBand::Light => [66, 142, 166],
        super::RiverBand::Mid => [50, 124, 156],
        super::RiverBand::Dark => [36, 104, 146],
    }
}

#[cfg(test)]
mod tests;
