//! Personal names: given name, family name by culture, optional byname.
//!
//! Rules:
//! - A given name is one or two elements (lexicon meanings such as "wolf"
//!   or "bright", or old meaningless name roots) plus a sex ending.
//! - The family name follows the culture's weighted custom: patronymic
//!   (father + son/daughter), occupational (trade), toponymic (of the home
//!   place), a descriptive compound, or kin of an ancestor. A household
//!   passes its family name down through [`FamilyCtx::family`].
//! - Adults may carry a byname ("the Tall") at the culture's rate.

use crate::compose::{finish, join, part};
use crate::language::Language;
use crate::meaning::{Class, Meaning as M};
use crate::place::place_name;
use crate::preset::FamilyStyle;
use crate::rng::{hash_str, Rng};
use crate::types::{Name, Part, PlaceKind, PlaceSpec, SiteTag};
use serde::{Deserialize, Serialize};

/// Sex, for given-name endings and patronymics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sex {
    /// Female.
    Female,
    /// Male.
    Male,
}

/// What the family and circumstances of a person supply. Everything is
/// optional; missing pieces are generated from `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyCtx {
    /// Stable identity of the person (an NPC id).
    #[serde(with = "crate::ids")]
    pub id: u64,
    /// The father's given name, for patronymics.
    pub father: Option<Name>,
    /// An inherited family name, used as is.
    pub family: Option<Name>,
    /// The person's or household's trade, for occupational names.
    pub trade: Option<M>,
    /// Home settlement, for toponymic names.
    pub home: Option<Name>,
    /// Adults may carry bynames.
    pub adult: bool,
}

impl FamilyCtx {
    /// Context with only an id (adult, nothing inherited).
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            id,
            father: None,
            family: None,
            trade: None,
            home: None,
            adult: true,
        }
    }
}

/// A full personal name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonName {
    /// Given name.
    pub given: Name,
    /// Family name, if the culture gives one.
    pub family: Option<Name>,
    /// Family-name custom used (`None` when inherited).
    pub style: Option<FamilyStyle>,
    /// Byname ("the Tall"), native form.
    pub byname: Option<Name>,
    /// Given and family name as written ("Tessa da Velora").
    pub full: String,
    /// English reading ("Tessa, of Oakford, the Tall").
    pub gloss: String,
}

const NAME_MEANINGS: &[M] = &[
    M::Wolf,
    M::Bear,
    M::Raven,
    M::Eagle,
    M::Hart,
    M::Boar,
    M::Horse,
    M::Swan,
    M::Hawk,
    M::Stone,
    M::Iron,
    M::Gold,
    M::Silver,
    M::Oak,
    M::Ash,
    M::Thorn,
    M::Star,
    M::Sun,
    M::Moon,
    M::Dawn,
    M::Song,
    M::Fire,
    M::Wind,
    M::Snow,
    M::Friend,
    M::Peace,
    M::War,
    M::Glory,
    M::Hope,
    M::Heart,
    M::Hand,
    M::Shield,
    M::Spear,
    M::Hammer,
    M::Bright,
    M::Fair,
    M::Wise,
    M::Bold,
    M::Hard,
    M::Strong,
    M::Keen,
    M::Holy,
    M::Red,
    M::White,
    M::Swift,
    M::King,
    M::Lord,
    M::Guard,
];
const COMPOUND_FIRST: &[M] = &[
    M::Stone,
    M::Iron,
    M::Oak,
    M::Ash,
    M::Thorn,
    M::Star,
    M::Moon,
    M::Wind,
    M::Fire,
    M::Gold,
    M::Silver,
    M::Red,
    M::Black,
    M::White,
    M::Grey,
    M::Bright,
    M::Deep,
    M::High,
    M::Swift,
    M::Strong,
    M::Hard,
    M::Wolf,
    M::Raven,
    M::Willow,
    M::Apple,
    M::Dawn,
    M::Snow,
    M::Copper,
    M::Salt,
    M::Green,
];
const COMPOUND_SECOND: &[M] = &[
    M::Hand,
    M::Heart,
    M::Shield,
    M::Hammer,
    M::Stone,
    M::Oak,
    M::Wolf,
    M::Spear,
    M::Horn,
    M::Song,
    M::Star,
    M::Thorn,
    M::Wind,
    M::Fire,
    M::Bell,
    M::Hill,
    M::Field,
    M::Brook,
    M::Wood,
    M::Ford,
];
const BYNAMES: &[M] = &[
    M::Tall,
    M::Young,
    M::Old,
    M::Quiet,
    M::Merry,
    M::Grim,
    M::Swift,
    M::Strong,
    M::Wise,
    M::Bold,
    M::Fair,
    M::Keen,
    M::Lucky,
    M::Stout,
    M::Red,
    M::Black,
    M::White,
    M::Grey,
    M::Wolf,
    M::Raven,
    M::Hammer,
    M::Bear,
];

/// A meaningless personal-name root, glossed by its own spelling.
pub(crate) fn name_root_part(lang: &Language, rng: &mut Rng) -> Part {
    let ph = rng.pick(&lang.name_roots).cloned().unwrap_or_default();
    Part {
        meaning: None,
        gloss: lang.spell(&lang.speak(&ph)),
        ph,
    }
}

/// A founder's name for a place: one or two name elements, no ending,
/// with its spelling as the English form ("Kelmar's town").
pub(crate) fn founder(lang: &Language, rng: &mut Rng) -> (Vec<Part>, String) {
    let first = name_root_part(lang, rng);
    let short = lang.phonology.syllables(&first.ph).len() < 2;
    let mut parts = vec![first];
    if short && rng.chance(650) {
        parts.push(element(lang, rng));
    }
    let word = join(&lang.phonology, &parts);
    let gloss = lang.spell(&lang.speak(&word));
    (
        vec![Part {
            meaning: None,
            gloss: gloss.clone(),
            ph: word,
        }],
        gloss,
    )
}

fn element(lang: &Language, rng: &mut Rng) -> Part {
    if rng.chance(450) {
        let m = rng.pick(NAME_MEANINGS).copied().unwrap_or(M::Bright);
        part(lang, m)
    } else {
        name_root_part(lang, rng)
    }
}

fn given_attempt(lang: &Language, sex: Sex, rng: &mut Rng) -> Option<Name> {
    let first = element(lang, rng);
    let long = lang.phonology.syllables(&first.ph).len() >= 2;
    let mut parts = vec![first];
    if !long && rng.chance(450) {
        parts.push(element(lang, rng));
    }
    let endings = &lang.endings[usize::from(sex == Sex::Male)];
    let ending = rng.pick(endings).cloned().unwrap_or_default();
    let meaning = if sex == Sex::Male {
        M::Masculine
    } else {
        M::Feminine
    };
    let meaningful: Vec<&Part> = parts.iter().filter(|p| p.meaning.is_some()).collect();
    let all_meaningful = meaningful.len() == parts.len();
    let literal = if meaningful.is_empty() {
        "(an old name)".to_string()
    } else {
        meaningful
            .iter()
            .filter_map(|p| p.meaning)
            .map(|m| m.info().word)
            .collect::<Vec<_>>()
            .join(" + ")
    };
    let gloss = parts
        .iter()
        .map(|p| p.gloss.to_ascii_lowercase())
        .collect::<String>();
    parts.push(Part {
        meaning: Some(meaning),
        gloss: String::new(),
        ph: ending,
    });
    let mut word = join(&lang.phonology, &parts);
    if parts.len() == 3 && lang.phonology.syllables(&word).len() > 3 {
        parts.remove(1);
        word = join(&lang.phonology, &parts);
    }
    let mut name = finish(lang, word, parts, &gloss, literal)?;
    if !all_meaningful {
        name.gloss = name.native.clone();
    } else {
        name.gloss = crate::ortho::capitalise(&name.gloss);
    }
    Some(name)
}

/// A given name for `key` (a person id or a salt).
#[must_use]
pub fn given_name(lang: &Language, sex: Sex, key: u64) -> Name {
    let mut rng = Rng::new(
        lang.seed,
        &[hash_str("given"), lang.preset as u64, key, sex as u64],
    );
    for _ in 0..24 {
        if let Some(n) = given_attempt(lang, sex, &mut rng) {
            return n;
        }
    }
    let p = part(lang, M::Friend);
    let word = p.ph.clone();
    Name {
        native: lang.spell(&lang.speak(&word)),
        gloss: "Friend".into(),
        literal: "friend".into(),
        pronunciation: lang.pronounce(&lang.speak(&word)),
        word,
        parts: vec![p],
    }
}

fn proper(name: &Name) -> Part {
    Part {
        meaning: None,
        gloss: name.native.clone(),
        ph: name.word.clone(),
    }
}

fn ordered(lang: &Language, affix: Part, base: Part) -> Vec<Part> {
    if lang.grammar.head_first {
        vec![affix, base]
    } else {
        vec![base, affix]
    }
}

fn family_attempt(
    lang: &Language,
    sex: Sex,
    ctx: &FamilyCtx,
    style: FamilyStyle,
    rng: &mut Rng,
) -> Option<Name> {
    let salt = rng.next_u64();
    match style {
        FamilyStyle::Patronymic | FamilyStyle::Clan => {
            let father = ctx
                .father
                .clone()
                .unwrap_or_else(|| given_name(lang, Sex::Male, salt));
            let (affix, lit) = match (style, sex) {
                (FamilyStyle::Clan, _) => (M::Kin, "kin of"),
                (_, Sex::Female) => (M::Daughter, "daughter of"),
                (_, Sex::Male) => (M::Son, "son of"),
            };
            let parts = ordered(lang, part(lang, affix), proper(&father));
            let literal = format!("{lit} {}", father.native);
            finish(
                lang,
                join(&lang.phonology, &parts),
                parts,
                &literal.clone(),
                literal,
            )
        }
        FamilyStyle::Occupational => {
            let trade = ctx.trade.filter(|t| t.info().class == Class::Trade);
            let trades: Vec<M> = M::of_class(Class::Trade).collect();
            let trade = trade
                .or_else(|| rng.pick(&trades).copied())
                .unwrap_or(M::Smith);
            let parts = vec![part(lang, trade)];
            let i = trade.info();
            finish(
                lang,
                join(&lang.phonology, &parts),
                parts,
                i.modifier,
                format!("the {}", i.word),
            )
        }
        FamilyStyle::Toponymic => {
            let home = ctx.home.clone().unwrap_or_else(|| {
                let kinds = [PlaceKind::Hamlet, PlaceKind::Village, PlaceKind::Town];
                let kind = rng.pick(&kinds).copied().unwrap_or(PlaceKind::Village);
                let tags = [
                    SiteTag::Ford,
                    SiteTag::Hill,
                    SiteTag::Forest,
                    SiteTag::Bridge,
                    SiteTag::Spring,
                    SiteTag::Marsh,
                    SiteTag::Arable,
                ];
                let tag = rng.pick(&tags).copied().unwrap_or(SiteTag::Arable);
                place_name(lang, &PlaceSpec::new(kind, salt).tags(&[tag]))
            });
            let gloss = format!("of {}", home.gloss);
            let literal = format!("of {}", home.native);
            if lang.grammar.head_first {
                let of = lang.ortho.render(&lang.speak(lang.root(M::Of)));
                let mut n = home.clone();
                n.native = format!("{of} {}", home.native);
                n.gloss = gloss;
                n.literal = literal;
                n.parts.insert(0, part(lang, M::Of));
                return (!lang.blocklist.blocks(&n.native)).then_some(n);
            }
            // Head-last tongues use the bare place name ("Ashford").
            let mut n = home;
            n.gloss = gloss;
            n.literal = literal;
            Some(n)
        }
        FamilyStyle::Compound => {
            let a = rng.pick(COMPOUND_FIRST).copied().unwrap_or(M::Stone);
            let b = rng
                .pick(COMPOUND_SECOND)
                .copied()
                .filter(|&b| b != a)
                .unwrap_or(M::Hand);
            let parts = if lang.grammar.head_first {
                vec![part(lang, b), part(lang, a)]
            } else {
                vec![part(lang, a), part(lang, b)]
            };
            let gloss = format!(
                "{}{}",
                a.info().modifier,
                b.info().word.to_ascii_lowercase()
            );
            let literal = format!("{} {}", a.info().word, b.info().word);
            finish(lang, join(&lang.phonology, &parts), parts, &gloss, literal)
        }
    }
}

/// Names a person. Deterministic in `(lang, sex, ctx)`.
#[must_use]
pub fn person_name(lang: &Language, sex: Sex, ctx: &FamilyCtx) -> PersonName {
    let given = given_name(lang, sex, ctx.id);
    let mut rng = Rng::new(
        lang.seed,
        &[hash_str("family"), lang.preset as u64, ctx.id, sex as u64],
    );
    let (family, style) = match &ctx.family {
        Some(f) => (Some(f.clone()), None),
        None => {
            let options = lang.family_weights();
            let weights: Vec<u32> = options.iter().map(|o| o.1).collect();
            let style = options
                .get(rng.weighted(&weights))
                .map_or(FamilyStyle::Occupational, |o| o.0);
            let f = (0..8).find_map(|_| family_attempt(lang, sex, ctx, style, &mut rng));
            (f, Some(style))
        }
    };
    let byname = if ctx.adult && rng.chance(lang.grammar.byname) {
        (0..8).find_map(|_| {
            let m = rng.pick(BYNAMES).copied().unwrap_or(M::Tall);
            let parts = vec![part(lang, m)];
            let g = format!("the {}", m.info().modifier);
            finish(lang, join(&lang.phonology, &parts), parts, &g.clone(), g)
        })
    } else {
        None
    };
    let full = match &family {
        Some(f) => format!("{} {}", given.native, f.native),
        None => given.native.clone(),
    };
    let mut gloss = if given.gloss == given.native {
        given.native.clone()
    } else {
        format!("{} ({})", given.native, given.gloss)
    };
    for extra in [&family, &byname].into_iter().flatten() {
        gloss.push_str(", ");
        gloss.push_str(&extra.gloss);
    }
    PersonName {
        given,
        family,
        style,
        byname,
        full,
        gloss,
    }
}
