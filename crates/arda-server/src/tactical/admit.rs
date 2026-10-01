//! Work admission for posted tactical layouts: limits the size checks do
//! not see (review round 2 #1–#3).

use crate::error::{ServerError, ServerResult};
use arda_tactical::TacticalLayout;

/// Longest layout name `POST /render` accepts, bytes. The name keys the
/// cache anchor and is echoed into logs and error messages.
pub const MAX_NAME_BYTES: usize = 128;
/// Squares a placement or light may sit outside the map (sprite overhang).
pub const POSITION_MARGIN_SQ: f32 = 16.0;
/// Placements allowed per square (plus [`PLACEMENT_SLACK`]).
pub const PLACEMENTS_PER_SQUARE: u64 = 4;
/// Placements allowed on top of the per-square allowance.
pub const PLACEMENT_SLACK: u64 = 64;
/// Lit pixels allowed per output pixel, summed over every free light.
pub const LIGHT_WORK_PER_PX: u64 = 16;

/// Refuses layouts whose size limits pass but whose work does not: long
/// names (cache-key and log bytes), unbounded placement counts, lights whose
/// summed pools cover the image many times over, and positions far off the
/// map (which overflow the compositor's pixel arithmetic). Refused, never
/// trimmed (goal-prompt §8).
pub(super) fn admit_work(layout: &TacticalLayout, ppsq: u32, px: u64) -> ServerResult<()> {
    if layout.name.len() > MAX_NAME_BYTES || layout.name.chars().any(char::is_control) {
        return Err(ServerError::InvalidLayout(format!(
            "layout names are at most {MAX_NAME_BYTES} bytes without control characters"
        )));
    }
    let squares = u64::from(layout.width) * u64::from(layout.height);
    let max_placements = squares * PLACEMENTS_PER_SQUARE + PLACEMENT_SLACK;
    if layout.placements.len() as u64 > max_placements {
        return Err(ServerError::PayloadTooLarge(format!(
            "{} placements; the limit for this map is {max_placements}",
            layout.placements.len()
        )));
    }
    let (w, h) = (layout.width as f32, layout.height as f32);
    let on_map = |x: f32, y: f32| {
        x.is_finite()
            && y.is_finite()
            && (-POSITION_MARGIN_SQ..=w + POSITION_MARGIN_SQ).contains(&x)
            && (-POSITION_MARGIN_SQ..=h + POSITION_MARGIN_SQ).contains(&y)
    };
    if let Some(i) = layout.placements.iter().position(|p| !on_map(p.x, p.y)) {
        return Err(ServerError::InvalidLayout(format!(
            "placement {i} lies more than {POSITION_MARGIN_SQ} squares off the map"
        )));
    }
    if let Some(i) = layout.lights.iter().position(|l| !on_map(l.x, l.y)) {
        return Err(ServerError::InvalidLayout(format!(
            "light {i} lies more than {POSITION_MARGIN_SQ} squares off the map"
        )));
    }
    // Each light visits its pool's box clipped to the image (compose/lighting).
    let (wpx, hpx) = (
        u64::from(layout.width) * u64::from(ppsq),
        u64::from(layout.height) * u64::from(ppsq),
    );
    let work: u64 = layout
        .lights
        .iter()
        .map(|l| {
            let side = 2 * u64::from(l.radius_ft) * u64::from(ppsq) / 5;
            side.min(wpx) * side.min(hpx)
        })
        .fold(0u64, u64::saturating_add);
    let budget = px.saturating_mul(LIGHT_WORK_PER_PX);
    if work > budget {
        return Err(ServerError::PayloadTooLarge(format!(
            "the lights would light {work} px; the limit is {budget}"
        )));
    }
    Ok(())
}
