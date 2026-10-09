//! Wall kits and floors from the settlement's own data (logic/10
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
/// the table of logic/10 §building-materials.
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

/// A building's standing within its own settlement: +1 well above the
/// settlement's wealth (≥ 45 more: manors, the best market houses), −1 well
/// below it (≥ 40 less: cottages, suburbs, poor barns), else 0. Relative,
/// so a rich town's ordinary houses are not all "wealthy" twice over.
#[must_use]
pub fn standing(fabric: &Fabric, b: &Building) -> i32 {
    let rel = i32::from(b.wealth) - i32::from(fabric.wealth);
    if rel >= 45 {
        1
    } else if rel <= -40 {
        -1
    } else {
        0
    }
}

/// The material of `b` in a settlement with `fabric` (logic/10
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
    let standing = standing(fabric, b);
    let finish = |m: Material| {
        if m == Material::Stone && standing < 0 {
            Material::Rubble
        } else {
            m
        }
    };
    if civic(b.function) {
        return Material::Stone;
    }
    if farm(b.function) {
        return if standing < 0 { poor } else { middling };
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
    rank += standing;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::grid::{Side, SquareRect};
    use crate::plan::BuildingId;
    use DistrictKind as D;
    use Material as M;
    use WealthLevel as W;

    fn house(id: u64, function: F, wealth_level: W) -> Building {
        Building {
            id: BuildingId(id),
            function,
            plot: None,
            footprint: Vec::new(),
            rect: SquareRect {
                x0: 0,
                y0: 0,
                x1: 5,
                y1: 8,
            },
            storeys: 2,
            wealth: 128,
            wealth_level,
            front: Side::North,
            doors: Vec::new(),
            ancillary: false,
            tags: Vec::new(),
        }
    }

    fn fabric(wealth: u8, merchant: bool, biome: &str) -> Fabric {
        Fabric {
            wealth,
            merchant,
            biome: biome.into(),
            ..Fabric::default()
        }
    }

    /// The material of `b`, its wealth set from its band relative to the
    /// settlement: poor 50 below, modest level, wealthy 50 above.
    fn of(f: &Fabric, tier: Tier, culture: &str, d: D, b: &Building) -> Material {
        let ctx = Context {
            district: d,
            inside_wall: true,
        };
        let mut b = b.clone();
        b.wealth = match b.wealth_level {
            W::Poor => f.wealth.saturating_sub(50),
            W::Modest => f.wealth,
            W::Wealthy => f.wealth.saturating_add(50),
        };
        material(f, tier, culture, &b, ctx, 42)
    }

    const DISTRICTS: [D; 7] = [
        D::Market,
        D::Residential,
        D::Craft,
        D::Waterfront,
        D::Religious,
        D::Castle,
        D::Suburb,
    ];
    const HOMES: [F; 6] = [
        F::House,
        F::Cottage,
        F::Workshop,
        F::Tavern,
        F::Inn,
        F::Bakery,
    ];

    #[test]
    fn a_rich_merchant_city_is_stone_throughout() {
        let f = fabric(250, true, "temperate_forest");
        for culture in ["heartland", "sylvan", "southern", "coastal"] {
            for d in DISTRICTS {
                for (i, func) in HOMES.iter().enumerate() {
                    for w in [W::Poor, W::Modest, W::Wealthy] {
                        let b = house(i as u64 + 1, *func, w);
                        let m = of(&f, Tier::City, culture, d, &b);
                        assert!(m.is_stone(), "{culture} {d:?} {func:?} {w:?}: {m:?}");
                        // Variation stays within stone: rubble for the poor.
                        assert_eq!(m == M::Rubble, w == W::Poor);
                    }
                }
            }
        }
        // Without merchants, or poorer, the same city splits by district.
        assert!(!stone_town(&fabric(250, false, "temperate"), Tier::City));
        assert!(!stone_town(&fabric(200, true, "temperate"), Tier::City));
        assert!(!stone_town(&fabric(250, true, "temperate"), Tier::Village));
    }

    #[test]
    fn otherwise_materials_divide_by_district() {
        // A heartland market town of wealth 183, like seed 42's Dilrou.
        let f = fabric(183, true, "wetland");
        let modest = |d: D, id: u64| {
            of(
                &f,
                Tier::Town,
                "heartland",
                d,
                &house(id, F::House, W::Modest),
            )
        };
        for id in 1..40 {
            assert_eq!(modest(D::Market, id), M::Stone);
            assert_eq!(modest(D::Religious, id), M::Stone);
            assert_eq!(modest(D::Residential, id), M::Timber);
            assert_eq!(modest(D::Suburb, id), M::Wattle);
        }
        // Artisan streets mix stone and timber, house by house.
        let craft: Vec<M> = (1..40).map(|id| modest(D::Craft, id)).collect();
        assert!(
            craft.contains(&M::Stone) && craft.contains(&M::Timber),
            "{craft:?}"
        );
        assert!(craft.iter().all(|m| matches!(m, M::Stone | M::Timber)));
        // A house's own wealth moves it one step.
        let rich = of(
            &f,
            Tier::Town,
            "heartland",
            D::Residential,
            &house(1, F::House, W::Wealthy),
        );
        let poor = of(
            &f,
            Tier::Town,
            "heartland",
            D::Residential,
            &house(1, F::House, W::Poor),
        );
        assert!(matches!(rich, M::Stone | M::Timber));
        assert_eq!(poor, M::Wattle);
        // Civic buildings are always stone.
        let temple = house(1, F::Temple, W::Poor);
        assert_eq!(
            of(&f, Tier::Town, "heartland", D::Suburb, &temple),
            M::Stone
        );
    }

    #[test]
    fn region_and_culture_set_the_vernacular() {
        let village = |f: &Fabric, culture: &str, w: W| {
            of(
                f,
                Tier::Village,
                culture,
                D::Farmstead,
                &house(1, F::Farmhouse, w),
            )
        };
        // A poor sylvan forest village: logs, wattle for the poorest.
        let mut forest = fabric(90, false, "temperate_forest");
        forest.forest = true;
        assert_eq!(village(&forest, "sylvan", W::Modest), M::Wattle);
        assert_eq!(village(&forest, "sylvan", W::Wealthy), M::Log);
        // The same village a little richer: logs for the middling.
        forest.wealth = 130;
        assert_eq!(village(&forest, "sylvan", W::Modest), M::Log);
        // Heartland timber; the steppe and the warm south build mud brick.
        assert_eq!(
            village(&fabric(130, false, "temperate"), "heartland", W::Modest),
            M::Timber
        );
        assert_eq!(
            village(&fabric(130, false, "steppe"), "heartland", W::Modest),
            M::Adobe
        );
        assert_eq!(
            village(&fabric(90, false, "warm_temperate"), "southern", W::Modest),
            M::Adobe
        );
        assert_eq!(
            village(&fabric(130, false, "boreal_forest"), "heartland", W::Modest),
            M::Log
        );
        // Building stone at hand lifts a village to stone.
        let mut hill = fabric(130, false, "temperate");
        hill.stone = true;
        let m = village(&hill, "heartland", W::Modest);
        assert!(matches!(m, M::Stone | M::Timber), "{m:?}");
    }

    #[test]
    fn history_rebuilds_in_stone() {
        let base = fabric(183, true, "wetland");
        let res = |f: &Fabric, id| {
            of(
                f,
                Tier::Town,
                "heartland",
                D::Residential,
                &house(id, F::House, W::Modest),
            )
        };
        let craft = |f: &Fabric, id| {
            of(
                f,
                Tier::Town,
                "heartland",
                D::Craft,
                &house(id, F::House, W::Modest),
            )
        };
        let mut burnt = base.clone();
        burnt.fires = 1;
        let mut golden = base.clone();
        golden.golden_age = true;
        let stone = |g: &dyn Fn(u64) -> M| (1..60).filter(|&id| g(id).is_stone()).count();
        assert_eq!(stone(&|id| res(&base, id)), 0);
        // After a great fire the residential streets are part stone, the
        // artisan streets all stone.
        assert!(stone(&|id| res(&burnt, id)) > 10);
        assert_eq!(stone(&|id| craft(&burnt, id)), 59);
        // A golden age rebuilds the core, not the residential streets.
        assert_eq!(stone(&|id| res(&golden, id)), 0);
        let market = of(
            &golden,
            Tier::Town,
            "heartland",
            D::Market,
            &house(1, F::House, W::Poor),
        );
        assert!(market.is_stone());
    }

    #[test]
    fn floors_follow_material_and_neighbours_differ() {
        assert_eq!(floor_palette(M::Stone, W::Wealthy)[0], "flagstone");
        assert_eq!(floor_palette(M::Wattle, W::Poor)[0], "packed_earth");
        assert_eq!(fixed_floor(F::Smithy), Some("packed_earth"));
        for name in crate::samples::NAMES {
            let (site, terrain) = crate::samples::by_name(name).unwrap();
            let plan = crate::generate(&site, &terrain, 42).unwrap();
            let looks = looks(&plan);
            let homes: Vec<usize> = (0..plan.buildings.len())
                .filter(|&i| fixed_floor(plan.buildings[i].function).is_none())
                .collect();
            let (mut pairs, mut same) = (0, 0);
            for &i in &homes {
                for &j in homes.iter().filter(|&&j| j > i) {
                    if touching(&plan.buildings[i].rect, &plan.buildings[j].rect) {
                        pairs += 1;
                        same += usize::from(looks[i].floor == looks[j].floor);
                    }
                }
            }
            assert!(
                same * 5 <= pairs,
                "{name}: {same} of {pairs} neighbours share a floor"
            );
            assert_eq!(looks, super::looks(&plan), "{name}: deterministic");
        }
    }
}
