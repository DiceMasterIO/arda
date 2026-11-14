//! The per-process memory ceiling (`--memory-mib`, `ARDA_MEMORY_MIB`;
//! logic/16 §api-memory).
//!
//! Without a ceiling the server admits its configured budgets against
//! [`crate::MEMORY_CEILING_BYTES`] and nothing changes. With one, every
//! budget the operator did not pin with a flag derives from it: each sits
//! at a floor (the least that still serves a cell at 64 px per square)
//! plus one shared fraction `s` of the way to its default, and `s` is the
//! largest that keeps [`ServerConfig::admitted_bytes`] plus what the world
//! and library hold ([`Resident`]) under the ceiling. Admission then
//! checks the same sum, so the worst case the server admits is the
//! ceiling's by construction.

use crate::admission::{PipelineLimits, Resident};
use crate::error::{ServerError, ServerResult};
use crate::tactical::cells::APRON;
use crate::ServerConfig;

/// Output pixels of one cell and its apron at 64 px per square: the
/// largest render a fitted server always admits (DiceMaster's cells).
pub const ONE_CELL_64_PX: u64 = {
    let side = (64 + 2 * APRON as u64) * 64;
    side * side
};

/// Budgets the operator set with a flag, which a ceiling leaves alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pinned {
    /// `--area-cache-mib`.
    pub area_cache: bool,
    /// `--fine-cache-mib`.
    pub fine_cache: bool,
    /// `--max-connections`.
    pub max_connections: bool,
    /// `--bake-result-mib`.
    pub bake_result: bool,
}

const MIB: u64 = 1 << 20;

/// Floors of the derived budgets. Each still serves its route: an area
/// cache of five decoded areas, a render cache of one 64-px cell render,
/// a pyramid cache of its pyramid, eight connections, two compositions.
mod floor {
    pub const AREA: usize = 64 << 20;
    pub const DERIVED: usize = 32 << 20;
    pub const FINE: usize = 32 << 20;
    pub const OVERVIEW: usize = 32 << 20;
    pub const RELIEF: usize = 32 << 20;
    pub const RENDER: usize = 96 << 20;
    pub const PYRAMID: usize = 128 << 20;
    pub const ENCODED: usize = 32 << 20;
    pub const CONNECTIONS: usize = 8;
    pub const AREA_BUILDS: usize = 1;
    // Two, as by default: with one, a request for a cell whose prefetch
    // has not finished waits behind the prefetch workers' compositions
    // (measured: p95 680 ms on the MICRO kit world at 4,608 MiB).
    pub const COMPOSE_SLOTS: usize = 2;
    pub const COMPOSED: usize = 32 << 20;
    pub const REFINED_BLOCKS: usize = 64;
    pub const PLANS: u64 = 128 << 20;
}

/// `lo + s × (hi − lo)`, rounded down; `hi` when `lo ≥ hi`.
fn lerp(lo: u64, hi: u64, s: f64) -> u64 {
    if lo >= hi {
        return hi;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let step = ((hi - lo) as f64 * s.clamp(0.0, 1.0)).floor() as u64;
    lo + step
}

/// `lerp` on byte budgets, rounded down to whole MiB.
fn lerp_mib(lo: usize, hi: usize, s: f64) -> usize {
    let b = lerp(lo as u64, hi as u64, s) / MIB * MIB;
    usize::try_from(b.max(lo as u64)).unwrap_or(lo)
}

fn lerp_count(lo: usize, hi: usize, s: f64) -> usize {
    usize::try_from(lerp(lo as u64, hi as u64, s)).unwrap_or(lo)
}

/// `config` with every unpinned budget at fraction `s` of the way from its
/// floor to its default.
fn at(config: &ServerConfig, s: f64) -> ServerConfig {
    let d = ServerConfig::new(config.world.clone());
    let mut c = config.clone();
    if !c.pinned.area_cache {
        c.query.area_bytes = lerp_mib(floor::AREA, d.query.area_bytes, s);
    }
    if !c.pinned.fine_cache {
        c.query.fine_bytes = lerp_mib(floor::FINE, d.query.fine_bytes, s);
    }
    c.query.derived_bytes = lerp_mib(floor::DERIVED, d.query.derived_bytes, s);
    c.overview.cache_bytes = lerp_mib(floor::OVERVIEW, d.overview.cache_bytes, s);
    c.relief.cache_bytes = lerp_mib(floor::RELIEF, d.relief.cache_bytes, s);
    let t = &mut c.tactical;
    t.render_cache_bytes = lerp_mib(floor::RENDER, d.tactical.render_cache_bytes, s);
    t.pyramid_cache_bytes = lerp_mib(floor::PYRAMID, d.tactical.pyramid_cache_bytes, s);
    t.encoded_cache_bytes = lerp_mib(floor::ENCODED, d.tactical.encoded_cache_bytes, s);
    t.max_render_px = lerp(ONE_CELL_64_PX, d.tactical.max_render_px, s);
    if !c.pinned.max_connections {
        c.serve.max_connections = lerp_count(floor::CONNECTIONS, d.serve.max_connections, s);
    }
    c.area_builds = lerp_count(floor::AREA_BUILDS, d.area_builds, s);
    let (p, dp) = (&mut c.pipeline, PipelineLimits::default());
    p.compose_slots = lerp_count(floor::COMPOSE_SLOTS, dp.compose_slots, s);
    p.composed_cache_bytes = lerp_mib(floor::COMPOSED, dp.composed_cache_bytes, s);
    p.refined_blocks = lerp_count(floor::REFINED_BLOCKS, dp.refined_blocks, s);
    p.plan_cache_bytes = lerp(floor::PLANS, dp.plan_cache_bytes, s) / MIB * MIB;
    if c.bake_workers > 0 && !c.pinned.bake_result {
        let one = crate::tactical::bake::worst_set_bytes().div_ceil(MIB);
        let lo = usize::try_from(one).unwrap_or(d.bake_result_mib);
        c.bake_result_mib = lerp_count(lo, d.bake_result_mib, s);
    }
    c
}

fn need(c: &ServerConfig, resident: &Resident) -> u64 {
    c.admitted_bytes().saturating_add(resident.total())
}

/// The least ceiling `config`'s flags admit with `resident`: every derived
/// budget at its floor.
#[must_use]
pub fn smallest(config: &ServerConfig, resident: &Resident) -> u64 {
    need(&at(config, 0.0), resident)
}

/// Fits `config` to its ceiling, if it has one (module docs); without one
/// it is returned unchanged.
///
/// # Errors
/// [`ServerError::ResourceLimit`] when the ceiling is below
/// [`smallest`], naming it in MiB.
pub fn fit(config: &ServerConfig, resident: &Resident) -> ServerResult<ServerConfig> {
    let Some(ceiling) = config.memory_ceiling else {
        return Ok(config.clone());
    };
    let least = smallest(config, resident);
    if least > ceiling {
        return Err(ServerError::ResourceLimit(format!(
            "--memory-mib {} is below the {} MiB this world, library and these flags need \
             at least (library {} MiB, society {} MiB, shore {} MiB, largest town plan {} MiB, \
             every derived budget at its floor)",
            ceiling / MIB,
            least.div_ceil(MIB),
            resident.library / MIB,
            resident.society / MIB,
            resident.shore / MIB,
            resident.plans / MIB,
        )));
    }
    let full = at(config, 1.0);
    let top = need(&full, resident);
    if top <= ceiling {
        return Ok(full);
    }
    // The admitted sum is affine in `s` up to rounding, so start at the
    // interpolated fraction and step down until it fits; `s = 0` fits.
    #[allow(clippy::cast_precision_loss)]
    let mut s = (ceiling - least) as f64 / (top - least) as f64;
    loop {
        let c = at(config, s);
        if need(&c, resident) <= ceiling || s <= 0.0 {
            return Ok(c);
        }
        s = (s - 1.0 / 1024.0).max(0.0);
    }
}

/// The plan budget the town warm-up runs under, before it has measured
/// the largest plan: `config`'s fitted one without that plan, or the floor
/// when even that does not fit (the fit after the warm-up then refuses
/// with the full measurement).
#[must_use]
pub fn warm_plan_budget(config: &ServerConfig, before: &Resident) -> u64 {
    fit(config, before).map_or(floor::PLANS, |c| c.pipeline.plan_cache_bytes)
}

#[cfg(test)]
mod tests;
