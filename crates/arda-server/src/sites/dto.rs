//! Wire bodies of the site routes (logic/18 §site-serve, family
//! `site_format`).
//!
//! The level body is served from the domain `arda_dungeon::Dungeon`, as
//! `GET /v1/tactical/dungeon` is, so the inner level has the same bytes;
//! [`SiteLevel`] is its mirror for ts-rs and schemars, and the tests prove
//! it reads that JSON.

use super::{LevelRef, Site};
use crate::tactical::dto::EdgeAxisDto;
use crate::tactical::dungeon_dto::{DungeonDto, DungeonExitKindDto, DungeonKindDto};
use arda_dungeon::site::{Link, LinkTo};
use arda_dungeon::traps::{Trap, TrapKind};
use arda_dungeon::{ExitKind, Kind};
use arda_tactical::layout::EdgeAxis;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Version of the site bodies.
pub const SITE_FORMAT: u32 = 1;

/// `GET /v1/sites`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct SiteList {
    /// Version of the site bodies ([`SITE_FORMAT`]).
    pub format_version: u32,
    /// The sites, by id.
    pub sites: Vec<SiteDto>,
}

/// A dungeon or cave site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename = "Site")]
pub struct SiteDto {
    /// Stable id, a decimal string.
    pub id: String,
    /// Dungeon or cave.
    pub kind: DungeonKindDto,
    /// The entrance square in world squares (64 per cell), `[x, y]`.
    #[ts(type = "[number, number]")]
    pub entrance_gs: [i64; 2],
    /// The entrance cell `[gx, gy]`.
    pub cell: [u32; 2],
    /// The settlement a dungeon hangs on, a decimal string; null for a cave.
    pub settlement_id: Option<String>,
    /// 1 to 3 levels, in depth order.
    pub levels: Vec<SiteLevelRef>,
}

/// One level of a site, as the site record names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct SiteLevelRef {
    /// 1-based depth.
    pub level: u32,
    /// Seed, a decimal string.
    pub seed: String,
    /// Width in squares: 32, 48 or 72.
    pub width: u32,
    /// Height in squares: 24, 36 or 56.
    pub height: u32,
    /// SRD 5.1 trap-table character-level band, 1 to 4.
    pub danger_tier: u8,
}

/// `GET /v1/sites/{id}/levels/{n}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
pub struct SiteLevel {
    /// The site.
    pub site_id: String,
    /// 1-based depth.
    pub level: u32,
    /// SRD level band, 1 to 4.
    pub danger_tier: u8,
    /// The level, as `GET /v1/tactical/dungeon` serves it for the level's
    /// seed, kind and size.
    pub dungeon: DungeonDto,
    /// One link per exit, in `dungeon.exits` order.
    pub links: Vec<SiteLink>,
    /// The level's traps.
    pub traps: Vec<SiteTrap>,
}

/// Where a link leads. Closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SiteLinkTo {
    /// The site's entrance square on the surface.
    Surface,
    /// Another level of the site.
    Level,
    /// Nowhere: the deepest level's stairs down.
    Sealed,
}

/// One exit of a level and where it leads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct SiteLink {
    /// The exit's kind.
    pub kind: DungeonExitKindDto,
    /// The exit's column.
    pub x: u32,
    /// The exit's row.
    pub y: u32,
    /// Where it leads.
    pub to: SiteLinkTo,
    /// Destination level (1-based), when `to` is `level`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub level: Option<u32>,
    /// Arrival column there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub to_x: Option<u32>,
    /// Arrival row there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub to_y: Option<u32>,
}

/// The SRD 5.1 sample traps. Closed. `fire_breathing_statue`,
/// `rolling_sphere` and `sphere_of_annihilation` are never placed under
/// `site_format` 1: placing them bumps the family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SiteTrapKind {
    /// Collapsing roof.
    CollapsingRoof,
    /// Falling net.
    FallingNet,
    /// Fire-breathing statue (not placed).
    FireBreathingStatue,
    /// Simple pit.
    SimplePit,
    /// Hidden pit.
    HiddenPit,
    /// Locking pit.
    LockingPit,
    /// Spiked pit.
    SpikedPit,
    /// Poison darts.
    PoisonDarts,
    /// Poison needle, in a locked door's lock.
    PoisonNeedle,
    /// Rolling sphere (not placed).
    RollingSphere,
    /// Sphere of annihilation (not placed).
    SphereOfAnnihilation,
}

/// A door edge, as in `dungeon.doors`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct SiteTrapDoor {
    /// Square column.
    pub x: u32,
    /// Square row.
    pub y: u32,
    /// Which edge.
    pub axis: EdgeAxisDto,
}

/// One trap. The consumer resolves it from the SRD text of its kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct SiteTrap {
    /// Stable per level: `t1`, `t2`, ….
    pub id: String,
    /// Which SRD trap.
    pub kind: SiteTrapKind,
    /// The squares that trigger it, `[x, y]`; for a poison needle, the two
    /// squares its door stands between.
    pub squares: Vec<[u32; 2]>,
    /// The squares hit, on a falling net and a collapsing roof.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected: Vec<[u32; 2]>,
    /// The locked door whose lock holds a poison needle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub door: Option<SiteTrapDoor>,
}

impl From<&LevelRef> for SiteLevelRef {
    fn from(l: &LevelRef) -> Self {
        Self {
            level: l.level,
            seed: l.seed.to_string(),
            width: l.width,
            height: l.height,
            danger_tier: l.danger_tier,
        }
    }
}

impl From<&Site> for SiteDto {
    fn from(s: &Site) -> Self {
        Self {
            id: s.id.to_string(),
            kind: match s.kind {
                Kind::Dungeon => DungeonKindDto::Dungeon,
                Kind::Cave => DungeonKindDto::Cave,
            },
            entrance_gs: s.entrance_gs(),
            cell: [s.cell.0, s.cell.1],
            settlement_id: s.settlement.map(|id| id.to_string()),
            levels: s.levels.iter().map(SiteLevelRef::from).collect(),
        }
    }
}

impl From<&Link> for SiteLink {
    fn from(l: &Link) -> Self {
        let (to, level, to_x, to_y) = match l.to {
            LinkTo::Surface => (SiteLinkTo::Surface, None, None, None),
            LinkTo::Level { level, x, y } => (SiteLinkTo::Level, Some(level), Some(x), Some(y)),
            LinkTo::Sealed => (SiteLinkTo::Sealed, None, None, None),
        };
        Self {
            kind: match l.kind {
                ExitKind::StairsUp => DungeonExitKindDto::StairsUp,
                ExitKind::StairsDown => DungeonExitKindDto::StairsDown,
                ExitKind::Mouth => DungeonExitKindDto::Mouth,
            },
            x: l.x,
            y: l.y,
            to,
            level,
            to_x,
            to_y,
        }
    }
}

impl From<&Trap> for SiteTrap {
    fn from(t: &Trap) -> Self {
        let pair = |&(x, y): &(u32, u32)| [x, y];
        Self {
            id: t.id.clone(),
            kind: match t.kind {
                TrapKind::CollapsingRoof => SiteTrapKind::CollapsingRoof,
                TrapKind::FallingNet => SiteTrapKind::FallingNet,
                TrapKind::FireBreathingStatue => SiteTrapKind::FireBreathingStatue,
                TrapKind::SimplePit => SiteTrapKind::SimplePit,
                TrapKind::HiddenPit => SiteTrapKind::HiddenPit,
                TrapKind::LockingPit => SiteTrapKind::LockingPit,
                TrapKind::SpikedPit => SiteTrapKind::SpikedPit,
                TrapKind::PoisonDarts => SiteTrapKind::PoisonDarts,
                TrapKind::PoisonNeedle => SiteTrapKind::PoisonNeedle,
                TrapKind::RollingSphere => SiteTrapKind::RollingSphere,
                TrapKind::SphereOfAnnihilation => SiteTrapKind::SphereOfAnnihilation,
            },
            squares: t.squares.iter().map(pair).collect(),
            affected: t.affected.iter().map(pair).collect(),
            door: t.door.map(|(x, y, axis)| SiteTrapDoor {
                x,
                y,
                axis: match axis {
                    EdgeAxis::Horizontal => EdgeAxisDto::Horizontal,
                    EdgeAxis::Vertical => EdgeAxisDto::Vertical,
                },
            }),
        }
    }
}
