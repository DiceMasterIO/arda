//! Place names: settlements, natural features, regions and realms.
//!
//! Rules:
//! - A settlement's head comes from its strongest site tag (estuary →
//!   mouth, harbour → haven, ford → ford, ...) or else its kind (hamlet →
//!   stead, town → ton); its modifier from the caller's features, else from
//!   what grows and lives at such a site, else a founder's name.
//! - Major rivers and some mountains keep names from the substrate tongue.
//! - Regions add "shire" to their capital; realms join the capital's first
//!   element to "mark".

use crate::compose::{borrowed, compound, derived, part, Modifier};
use crate::language::Language;
use crate::meaning::{Class, Meaning as M};
use crate::person::founder;
use crate::rng::{hash_str, Rng};
use crate::types::{Name, PlaceKind as K, PlaceSpec, SiteTag as T};

/// Draws before giving up on a non-blocked name for one salt range.
const TRIES: u64 = 24;
/// From this draw on, settlements are named after founders, whose names
/// are practically inexhaustible (used when a scope is crowded).
pub(crate) const FOUNDER_SALT: u64 = 12;
/// Syllables above which early draws are rejected as unwieldy.
const LONG: usize = 4;

const FOREST: &[M] = &[
    M::Oak,
    M::Ash,
    M::Birch,
    M::Pine,
    M::Yew,
    M::Thorn,
    M::Hart,
    M::Boar,
    M::Wolf,
    M::Green,
    M::Dark,
    M::Bear,
    M::Raven,
    M::Wild,
];
const MARSH: &[M] = &[
    M::Reed,
    M::Willow,
    M::Heron,
    M::Still,
    M::Black,
    M::Grey,
    M::Swan,
    M::Cold,
];
const COAST: &[M] = &[
    M::Gull,
    M::Salt,
    M::Grey,
    M::White,
    M::Wind,
    M::Swan,
    M::Far,
    M::Fish,
    M::Dawn,
    M::Star,
    M::Bright,
];
const HIGH: &[M] = &[
    M::Stone,
    M::White,
    M::Grey,
    M::High,
    M::Wind,
    M::Eagle,
    M::Raven,
    M::Cold,
    M::Snow,
    M::Red,
    M::Iron,
    M::Hawk,
    M::Giant,
    M::Dragon,
    M::Black,
    M::Horn,
];
const WATER: &[M] = &[
    M::Willow,
    M::Swan,
    M::Clear,
    M::Swift,
    M::Broad,
    M::Deep,
    M::Stone,
    M::Ash,
    M::Reed,
    M::Heron,
    M::Bright,
    M::Cold,
    M::Still,
    M::Black,
    M::Grey,
    M::Green,
];
const ORE: &[M] = &[
    M::Iron,
    M::Copper,
    M::Silver,
    M::Gold,
    M::Red,
    M::Black,
    M::Deep,
    M::Grey,
];
const FARM: &[M] = &[
    M::Oak,
    M::Ash,
    M::Apple,
    M::Barley,
    M::Sheep,
    M::Cattle,
    M::Horse,
    M::Bee,
    M::Green,
    M::Long,
    M::Broad,
    M::Old,
    M::New,
    M::High,
    M::Fair,
    M::Little,
    M::Great,
    M::Sun,
    M::Goat,
    M::Thorn,
    M::Birch,
    M::Stone,
    M::Red,
    M::White,
];
const HOLY: &[M] = &[
    M::Holy,
    M::Monk,
    M::Bell,
    M::Cross,
    M::Star,
    M::White,
    M::Dawn,
    M::Peace,
    M::Song,
    M::Sun,
    M::Moon,
];
const FORT: &[M] = &[
    M::King,
    M::Lord,
    M::Guard,
    M::Stone,
    M::Iron,
    M::Red,
    M::High,
    M::Wolf,
    M::Raven,
    M::Tower,
    M::Wall,
    M::Black,
    M::Hard,
];
const CITY: &[M] = &[
    M::King,
    M::Queen,
    M::Lord,
    M::New,
    M::Old,
    M::Great,
    M::High,
    M::White,
    M::Golden,
    M::Star,
    M::Sun,
    M::Bell,
    M::Tower,
    M::Bright,
    M::Fair,
    M::Cross,
];
const RIVER_OLD: &[M] = &[
    M::Bright,
    M::Swift,
    M::Clear,
    M::Dark,
    M::Deep,
    M::Cold,
    M::Grey,
    M::White,
    M::Still,
    M::Broad,
    M::Black,
    M::Swan,
    M::Willow,
    M::Moon,
    M::Star,
    M::Salt,
    M::Wolf,
    M::Stone,
    M::Green,
    M::Red,
];
/// Distinguishers for names that must differ from a neighbour's.
pub(crate) const DISTINGUISH: &[M] = &[
    M::Upper,
    M::Lower,
    M::Little,
    M::Great,
    M::North,
    M::South,
    M::East,
    M::West,
    M::New,
    M::Old,
];

/// Head meaning a site tag suggests, strongest first.
const TAG_HEADS: &[(T, M)] = &[
    (T::Estuary, M::Mouth),
    (T::Harbour, M::Harbour),
    (T::Confluence, M::Meet),
    (T::Ford, M::Ford),
    (T::Bridge, M::Bridge),
    (T::Pass, M::Pass),
    (T::Defensible, M::Fort),
    (T::Spring, M::Spring),
    (T::Salt, M::Saltworks),
    (T::Ore, M::Mine),
    (T::Lake, M::Lake),
    (T::Marsh, M::Marsh),
    (T::Coast, M::Strand),
    (T::Hill, M::Hill),
    (T::Mountain, M::Crag),
    (T::Forest, M::Grove),
    (T::River, M::Mill),
    (T::Timber, M::Wood),
    (T::Arable, M::Field),
];

fn tag_pool(t: T) -> &'static [M] {
    match t {
        T::Forest | T::Timber => FOREST,
        T::Marsh => MARSH,
        T::Coast | T::Harbour | T::Estuary | T::Fish => COAST,
        T::Hill | T::Mountain | T::Pass | T::Defensible => HIGH,
        T::Ford | T::Bridge | T::Confluence | T::Spring | T::Lake | T::River | T::Navigable => {
            WATER
        }
        T::Ore => ORE,
        T::Salt => &[M::Salt, M::White],
        T::Arable => FARM,
    }
}

fn kind_pool(k: K) -> &'static [M] {
    match k {
        K::Town | K::City => CITY,
        K::Fort => FORT,
        K::Abbey => HOLY,
        K::Port | K::Bay | K::Cape | K::Island => COAST,
        K::Mine => ORE,
        K::Mountain | K::Hill | K::Pass | K::Vale => HIGH,
        K::Forest => FOREST,
        K::Marsh => MARSH,
        K::River | K::Stream | K::Lake => WATER,
        _ => FARM,
    }
}

fn kind_head(k: K) -> M {
    match k {
        K::Hamlet => M::Farm,
        K::Village => M::Village,
        K::Town | K::City => M::Town,
        K::Fort => M::Fort,
        K::Abbey => M::Shrine,
        K::Port => M::Harbour,
        K::Mine => M::Mine,
        K::River => M::River,
        K::Stream => M::Brook,
        K::Lake => M::Lake,
        K::Mountain => M::Mount,
        K::Hill => M::Hill,
        K::Forest => M::Wood,
        K::Marsh => M::Marsh,
        K::Pass => M::Pass,
        K::Bay => M::Bay,
        K::Island => M::Island,
        K::Cape => M::Cape,
        K::Vale => M::Vale,
        K::Region => M::Land,
        K::Realm => M::Realm,
    }
}

fn choose_head(spec: &PlaceSpec, rng: &mut Rng) -> M {
    if !spec.kind.is_settlement() {
        if spec.kind == K::Mountain && rng.chance(300) {
            return M::Crag;
        }
        if spec.kind == K::Forest && rng.chance(300) {
            return M::Grove;
        }
        return kind_head(spec.kind);
    }
    let mut options: Vec<(M, u32)> = TAG_HEADS
        .iter()
        .filter(|(t, _)| spec.site_tags.contains(t))
        .zip(
            [12_u32, 4, 2, 1, 1, 1]
                .into_iter()
                .chain(std::iter::repeat(1)),
        )
        .map(|((_, m), w)| (*m, w))
        .collect();
    let kind_weight = if matches!(spec.kind, K::Town | K::City | K::Abbey | K::Fort) {
        6
    } else {
        4
    };
    options.push((kind_head(spec.kind), kind_weight));
    if matches!(spec.kind, K::Village | K::Hamlet) {
        options.push((
            if spec.kind == K::Hamlet {
                M::Village
            } else {
                M::Farm
            },
            1,
        ));
    }
    let weights: Vec<u32> = options.iter().map(|o| o.1).collect();
    options
        .get(rng.weighted(&weights))
        .map_or(M::Village, |o| o.0)
}

fn choose_modifier(
    lang: &Language,
    spec: &PlaceSpec,
    head: M,
    rng: &mut Rng,
    salt: u64,
) -> Modifier {
    let forced = salt >= FOUNDER_SALT && spec.kind.is_settlement();
    if !forced && !spec.features.is_empty() && rng.chance(850) {
        let n = spec.features.len().min(2);
        if let Some(&m) = spec.features.get(rng.below(n)) {
            return Modifier::Meaning(m);
        }
    }
    let generic = matches!(head, M::Farm | M::Town | M::Village);
    if forced || (generic && spec.kind.is_settlement() && rng.chance(250)) {
        let (parts, gloss) = founder(lang, rng);
        return Modifier::Proper(parts, gloss);
    }
    let mut pool: Vec<M> = kind_pool(spec.kind).to_vec();
    for &t in &spec.site_tags {
        pool.extend_from_slice(tag_pool(t));
        pool.extend_from_slice(tag_pool(t));
    }
    pool.retain(|&m| m != head && !(head == M::Mill && m == M::Mill));
    Modifier::Meaning(rng.pick(&pool).copied().unwrap_or(M::Old))
}

fn wants_plural(modi: &Modifier, rng: &mut Rng) -> bool {
    match modi {
        Modifier::Meaning(m) => {
            let i = m.info();
            !i.plural.is_empty() && i.plural != i.word && i.class == Class::Noun && rng.chance(350)
        }
        Modifier::Proper(..) => false,
    }
}

/// One attempt at a name; `None` when the draw was blocked or, in early
/// draws, longer than [`LONG`] syllables (regions and realms excepted).
pub(crate) fn attempt(lang: &Language, spec: &PlaceSpec, salt: u64) -> Option<Name> {
    let name = draw(lang, spec, salt)?;
    let derived = matches!(spec.kind, K::Region | K::Realm);
    if !derived && salt < TRIES / 2 && lang.phonology.syllables(&name.word).len() > LONG {
        return None;
    }
    Some(name)
}

fn draw(lang: &Language, spec: &PlaceSpec, salt: u64) -> Option<Name> {
    let mut rng = Rng::new(
        lang.seed,
        &[
            hash_str("place"),
            lang.preset as u64,
            spec.key,
            spec.kind as u64,
            salt,
        ],
    );
    match spec.kind {
        K::Region => {
            if let Some(base) = &spec.from {
                let literal = format!("shire of {}", base.gloss);
                return derived(lang, base, M::Shire, "shire", literal);
            }
        }
        K::Realm => {
            if let Some(base) = &spec.from {
                let first = base.parts.iter().find(|p| {
                    p.meaning.is_none_or(|m| {
                        matches!(m.info().class, Class::Noun | Class::Adj | Class::Being)
                    })
                });
                let first = first.or(base.parts.first())?;
                let modi = Modifier::Proper(vec![first.clone()], first.gloss.clone());
                let mut n = compound(lang, &modi, M::Realm, false)?;
                n.literal = format!("realm of {}", base.gloss);
                return Some(n);
            }
        }
        K::River if rng.chance(800) => {
            let adj = spec.features.first().copied().filter(|_| rng.chance(500));
            let adj = adj
                .or_else(|| rng.pick(RIVER_OLD).copied())
                .unwrap_or(M::Bright);
            let simplex = rng.chance(450);
            return borrowed(lang, adj, M::River, "water", simplex);
        }
        K::Mountain if rng.chance(300) => {
            let adj = rng.pick(HIGH).copied().unwrap_or(M::White);
            return borrowed(lang, adj, M::Mount, "peak", false);
        }
        _ => {}
    }
    let head = choose_head(spec, &mut rng);
    let modi = choose_modifier(lang, spec, head, &mut rng, salt);
    let plural = wants_plural(&modi, &mut rng);
    compound(lang, &modi, head, plural)
}

/// Names a place. Deterministic in `(lang, spec)`; never returns a name
/// the language's blocklist rejects.
#[must_use]
pub fn place_name(lang: &Language, spec: &PlaceSpec) -> Name {
    for salt in 0..TRIES {
        if let Some(n) = attempt(lang, spec, salt) {
            return n;
        }
    }
    fallback(lang, spec)
}

/// The plain head word, glossed by itself. Lexicon roots are never blocked
/// (`Language::fresh_root`), but a dialect's sound changes can respell one
/// into a blocked word; then the root is used as it stands (review round 2
/// #42: the fallback skipped the blocklist).
fn fallback(lang: &Language, spec: &PlaceSpec) -> Name {
    let head = kind_head(spec.kind);
    let p = part(lang, head);
    let changed = lang.speak(&p.ph);
    let spoken = if lang.blocklist.blocks(&lang.spell(&changed)) {
        p.ph.clone()
    } else {
        changed
    };
    Name {
        native: lang.spell(&spoken),
        gloss: head.info().modifier.to_string(),
        literal: head.info().word.to_string(),
        pronunciation: lang.pronounce(&spoken),
        word: p.ph.clone(),
        parts: vec![p],
    }
}

/// `name` with a distinguishing adjective in front ("Upper Oakford").
pub(crate) fn distinguished(lang: &Language, name: &Name, adj: M) -> Option<Name> {
    let mut parts = name.parts.clone();
    if lang.grammar.head_first {
        parts.push(part(lang, adj));
    } else {
        parts.insert(0, part(lang, adj));
    }
    let word = crate::compose::join(&lang.phonology, &parts);
    let gloss = format!("{} {}", adj.info().modifier, name.gloss);
    let literal = format!("{} {}", adj.info().word, name.literal);
    crate::compose::finish(lang, word, parts, &gloss, literal)
}
