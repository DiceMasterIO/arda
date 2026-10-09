//! Wall kits and floors from the settlement's own data (logic/09
//! §building-materials), using the shared vocabulary keys (`stone`,
//! `drystone`, `timber`, `wattle`, `log`, `adobe`; `planks`,
//! `packed_earth`, `stone_floor`, `flagstone`).
//!
//! Materials are never drawn per house at random. A settlement's culture
//! and biome give its vernacular (what the poor and the middling build
//! with); its wealth, tier, trade, building stone and history give each
//! district a rank; a house's own wealth nudges the rank by one. Rank picks
//! the material: rubble or ashlar stone at the top, a stone/vernacular mix
//! below, then the middling and the poor vernacular. A rich merchant town
//! or city is stone throughout, varied only by stone finish and floors.
//! Floors follow the material and wealth, and neighbours that share a wall
//! take different floors where the palette allows, so a terraced row shows
//! where one house ends.

use crate::function::BuildingFunction as F;
use crate::plan::{Building, DistrictKind, Fabric, TownPlan, WealthLevel};
use crate::rng::{hash, hash_str, unit};
use crate::site::Tier;

/// How a culture builds ordinary houses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tradition {
    /// Timber framing; stone only for the rich and for civic buildings.
    Timber,
    /// Stone throughout; rubble for the poorest.
    Stone,
    /// Wattle and daub for the poor, timber (or logs in woodland) for the
    /// rest.
    Wattle,
}

/// Building tradition of an `arda-settle` culture key; unknown cultures
/// pick one by hash so they stay stable.
#[must_use]
pub fn tradition(culture: &str) -> Tradition {
    match culture {
        "heartland" | "coastal" => Tradition::Timber,
        "highland" | "southern" => Tradition::Stone,
        "sylvan" | "borderland" => Tradition::Wattle,
        other => match hash_str(0x7ad, other) % 3 {
            0 => Tradition::Timber,
            1 => Tradition::Stone,
            _ => Tradition::Wattle,
        },
    }
}

/// A building material, one wall kit each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Material {
    /// Dressed ashlar.
    Stone,
    /// Rubble stone.
    Rubble,
    /// Timber framing.
    Timber,
    /// Wattle and daub.
    Wattle,
    /// Horizontal logs.
    Log,
    /// Sun-dried mud brick.
    Adobe,
}

impl Material {
    /// Wall kit key.
    #[must_use]
    pub const fn kit(self) -> &'static str {
        match self {
            Self::Stone => "stone",
            Self::Rubble => "drystone",
            Self::Timber => "timber",
            Self::Wattle => "wattle",
            Self::Log => "log",
            Self::Adobe => "adobe",
        }
    }

    /// Masonry of either finish.
    #[must_use]
    pub const fn is_stone(self) -> bool {
        matches!(self, Self::Stone | Self::Rubble)
    }
}

/// The vernacular `(poor, middling)` materials of a tradition in a biome:
/// the table of logic/09 §building-materials.
#[must_use]
pub fn vernacular(t: Tradition, biome: &str, forest: bool) -> (Material, Material) {
    use Material as M;
    match (biome, t) {
        // Treeless grassland: mud brick for everyone.
        ("steppe", _) => (M::Adobe, M::Adobe),
        // The northern forest: log houses.
        ("boreal_forest", _) => (M::Log, M::Log),
        // Uplands: rubble and stone.
        ("highland" | "alpine", _) => (M::Rubble, M::Stone),
        ("warm_temperate", Tradition::Stone) => (M::Adobe, M::Stone),
        (_, Tradition::Stone) => (M::Rubble, M::Stone),
        (_, Tradition::Wattle) if forest => (M::Wattle, M::Log),
        (_, Tradition::Wattle | Tradition::Timber) => (M::Wattle, M::Timber),
    }
}

/// Base rank of a district: the market core, the castle and the temple
/// close build best, suburbs worst.
#[must_use]
pub const fn district_rank(d: DistrictKind) -> i32 {
    match d {
        DistrictKind::Castle => 4,
        DistrictKind::Market | DistrictKind::Religious => 3,
        DistrictKind::Waterfront | DistrictKind::Craft | DistrictKind::Farmstead => 2,
        DistrictKind::Residential => 1,
        DistrictKind::Suburb => 0,
    }
}

/// A core district, where long prosperity rebuilds in stone first.
const fn core(d: DistrictKind) -> bool {
    matches!(
        d,
        DistrictKind::Castle | DistrictKind::Market | DistrictKind::Religious
    )
}

/// The settlement-wide rank modifier: wealth band, tier and building stone.
#[must_use]
pub fn settlement_rank(f: &Fabric, tier: Tier) -> i32 {
    let wealth = match f.wealth {
        0..100 => -1,
        100..170 => 0,
        170..230 => 1,
        _ => 2,
    };
    let tier = match tier {
        Tier::City => 1,
        Tier::Town | Tier::Village => 0,
        Tier::Hamlet => -1,
    };
    wealth + tier + i32::from(f.stone)
}

/// A rich merchant town or city: stone throughout.
#[must_use]
pub fn stone_town(f: &Fabric, tier: Tier) -> bool {
    matches!(tier, Tier::Town | Tier::City) && f.merchant && f.wealth >= 220
}

/// Always-stone functions.
fn civic(f: F) -> bool {
    matches!(
        f,
        F::Temple
            | F::Keep
            | F::Library
            | F::School
            | F::Guardhouse
            | F::Barracks
            | F::MarketHall
            | F::Manor
    )
}

/// Farm buildings: always the vernacular.
fn farm(f: F) -> bool {
    matches!(f, F::Barn | F::Stable | F::Boathouse)
}

/// What decides one building's material.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// District (residential for off-plot buildings).
    pub district: DistrictKind,
    /// Inside the town wall.
    pub inside_wall: bool,
}

/// The material of `b` in a settlement with `fabric` (logic/09
/// §building-materials). Pure: the same inputs always give the same kit.
#[must_use]
pub fn material(
    fabric: &Fabric,
    tier: Tier,
    culture: &str,
    b: &Building,
    ctx: Context,
    seed: u64,
) -> Material {
    let (poor, middling) = vernacular(tradition(culture), &fabric.biome, fabric.forest);
    let finish = |m: Material| {
        if m == Material::Stone && b.wealth_level == WealthLevel::Poor {
            Material::Rubble
        } else {
            m
        }
    };
    if civic(b.function) {
        return Material::Stone;
    }
    if farm(b.function) {
        return if b.wealth_level == WealthLevel::Poor {
            poor
        } else {
            middling
        };
    }
    if stone_town(fabric, tier) {
        return finish(Material::Stone);
    }
    let mut rank = district_rank(ctx.district) + settlement_rank(fabric, tier);
    if fabric.golden_age && core(ctx.district) {
        rank += 1;
    }
    // A great fire: rebuilt in stone under the town's ordinances.
    if fabric.fires > 0
        && ctx.inside_wall
        && !matches!(ctx.district, DistrictKind::Suburb | DistrictKind::Farmstead)
    {
        rank += 1;
    }
    rank += match b.wealth_level {
        WealthLevel::Poor => -1,
        WealthLevel::Modest => 0,
        WealthLevel::Wealthy => 1,
    };
    match rank {
        4.. => finish(Material::Stone),
        3 => {
            if unit(hash(seed ^ 0x57_0E, b.id.0, 3)) < 0.5 {
                finish(Material::Stone)
            } else {
                middling
            }
        }
        2 => middling,
        _ => poor,
    }
}

/// The partition kit inside a building of material `m`.
#[must_use]
pub fn partition_kit(m: Material, wealth: WealthLevel) -> &'static str {
    match (m, wealth) {
        (Material::Adobe, _) => "adobe",
        (Material::Wattle, _) | (_, WealthLevel::Poor) => "wattle",
        _ => "timber",
    }
}

/// Floor palette of a house of material `m`, most fitting first.
#[must_use]
pub fn floor_palette(m: Material, wealth: WealthLevel) -> &'static [&'static str] {
    use WealthLevel as W;
    match (m, wealth) {
        (Material::Stone | Material::Rubble, W::Wealthy) => &["flagstone", "planks", "stone_floor"],
        (Material::Stone | Material::Rubble, W::Modest) => &["stone_floor", "planks", "flagstone"],
        (Material::Stone | Material::Rubble, W::Poor) => &["stone_floor", "packed_earth"],
        (Material::Timber | Material::Log, W::Wealthy) => &["planks", "flagstone"],
        (Material::Timber | Material::Log, W::Modest) => &["planks", "packed_earth"],
        (Material::Adobe, W::Wealthy) => &["flagstone", "stone_floor"],
        (Material::Adobe, _) => &["packed_earth", "stone_floor"],
        (_, W::Poor) => &["packed_earth", "planks"],
        _ => &["planks", "packed_earth"],
    }
}

/// The fixed floor of a working or civic building, or `None` for homes,
/// whose floor follows material and wealth.
#[must_use]
pub fn fixed_floor(f: F) -> Option<&'static str> {
    Some(match f {
        F::Temple | F::Shrine | F::Keep | F::Library | F::MarketHall => "flagstone",
        F::Warehouse | F::Barracks | F::Guardhouse | F::Mill | F::Bakery => "stone_floor",
        F::Smithy | F::Stable | F::Barn | F::Tannery | F::Brewery => "packed_earth",
        _ => return None,
    })
}

/// One building's wall kits and floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look {
    /// Shell material.
    pub material: Material,
    /// Shell (exterior and party wall) kit.
    pub shell: &'static str,
    /// Interior partition kit.
    pub partition: &'static str,
    /// Main floor.
    pub floor: &'static str,
}

/// The districts and wall sides of a plan's plots, by plot id.
fn context(plan: &TownPlan, b: &Building) -> Context {
    let plot = b.plot.and_then(|id| {
        let i = usize::try_from(id.0).ok()?;
        plan.plots
            .get(i)
            .filter(|p| p.id == id)
            .or_else(|| plan.plots.iter().find(|p| p.id == id))
    });
    let castle = matches!(b.function, F::Keep | F::Barracks);
    Context {
        district: plot.map_or(
            if castle {
                DistrictKind::Castle
            } else {
                DistrictKind::Residential
            },
            |p| p.district,
        ),
        inside_wall: plot.is_none_or(|p| p.inside_wall),
    }
}

/// Whether two footprints share at least one square edge.
const fn touching(a: &crate::plan::grid::SquareRect, b: &crate::plan::grid::SquareRect) -> bool {
    let x = a.x0 < b.x1 && b.x0 < a.x1;
    let y = a.y0 < b.y1 && b.y0 < a.y1;
    (x && (a.y1 == b.y0 || b.y1 == a.y0)) || (y && (a.x1 == b.x0 || b.x1 == a.x0))
}

/// Every building's look, by position. Floors are coloured greedily in id
/// order: each house takes the first floor of its palette, starting at a
/// hashed offset, that no lower-id neighbour sharing a wall already has.
#[must_use]
pub fn looks(plan: &TownPlan) -> Vec<Look> {
    let n = plan.buildings.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| plan.buildings[i].rect.x0);
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (k, &i) in order.iter().enumerate() {
        let a = plan.buildings[i].rect;
        for &j in &order[k + 1..] {
            let b = plan.buildings[j].rect;
            if b.x0 > a.x1 {
                break;
            }
            if touching(&a, &b) {
                neighbours[i].push(j);
                neighbours[j].push(i);
            }
        }
    }
    let mut out: Vec<Look> = Vec::with_capacity(n);
    for (i, b) in plan.buildings.iter().enumerate() {
        let m = material(
            &plan.fabric,
            plan.tier,
            &plan.culture,
            b,
            context(plan, b),
            plan.seed,
        );
        let floor = fixed_floor(b.function).unwrap_or_else(|| {
            let palette = floor_palette(m, b.wealth_level);
            let taken: Vec<&str> = neighbours[i]
                .iter()
                .filter(|&&j| j < i)
                .map(|&j| out[j].floor)
                .collect();
            // Most houses keep their best-fitting floor; one in four starts
            // further down the palette.
            let len = palette.len();
            let start = if unit(hash(plan.seed ^ 0xF1_00, b.id.0, 7)) < 0.25 {
                1 % len
            } else {
                0
            };
            (0..len)
                .map(|k| palette[(start + k) % len])
                .find(|f| !taken.contains(f))
                .unwrap_or(palette[start])
        });
        out.push(Look {
            material: m,
            shell: m.kit(),
            partition: partition_kit(m, b.wealth_level),
            floor,
        });
    }
    out
}

/// The look of `b` (cached for the whole plan).
#[must_use]
pub fn look(plan: &TownPlan, b: &Building) -> Look {
    let all = plan.looks.get_or_init(|| looks(plan));
    usize::try_from(b.id.0.saturating_sub(1))
        .ok()
        .and_then(|i| all.get(i).copied())
        .filter(|_| plan.building(b.id).is_some_and(|x| x.rect == b.rect))
        .unwrap_or_else(|| {
            let m = material(
                &plan.fabric,
                plan.tier,
                &plan.culture,
                b,
                context(plan, b),
                plan.seed,
            );
            Look {
                material: m,
                shell: m.kit(),
                partition: partition_kit(m, b.wealth_level),
                floor: fixed_floor(b.function)
                    .unwrap_or_else(|| floor_palette(m, b.wealth_level)[0]),
            }
        })
}
