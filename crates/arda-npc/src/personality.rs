//! Personalities from Arda's original tables, combined coherently: two traits
//! on different axes, an ideal the job allows (no doubting priest, no
//! pacifist soldier) and a bond that names real people and places.

use crate::data::content::IdealEntry;
use crate::data::Data;
use crate::npc::{Ideal, Personality};
use crate::rng::Rng;

/// Weight of an ideal that suits the job, against 1 for any other allowed one.
const AFFINITY_WEIGHT: u32 = 4;

/// Names the bond templates can refer to.
pub(crate) struct BondContext<'a> {
    pub settlement: &'a str,
    pub workplace: Option<&'a str>,
    pub kin: Option<&'a str>,
    pub friend: Option<&'a str>,
    pub rival: Option<&'a str>,
}

/// Whether an ideal is allowed for a job category.
#[must_use]
pub fn ideal_allowed(data_exclusions: &[String], ideal_tags: &[String]) -> bool {
    !ideal_tags.iter().any(|t| data_exclusions.contains(t))
}

fn choose_ideal<'a>(data: &'a Data, category: &str, rng: &mut Rng) -> Option<&'a IdealEntry> {
    let tables = &data.personality;
    let empty = Vec::new();
    let excluded = tables.exclusions.get(category).unwrap_or(&empty);
    let liked = tables.affinities.get(category).unwrap_or(&empty);
    let weights: Vec<u32> = tables
        .ideals
        .iter()
        .map(|ideal| {
            if !ideal_allowed(excluded, &ideal.tags) {
                0
            } else if ideal.tags.iter().any(|t| liked.contains(t)) {
                AFFINITY_WEIGHT
            } else {
                1
            }
        })
        .collect();
    rng.weighted(&weights).and_then(|i| tables.ideals.get(i))
}

/// Generates a personality for someone of `category` working as `title`.
pub(crate) fn generate(
    data: &Data,
    category: &str,
    title: &str,
    ctx: &BondContext<'_>,
    rng: &mut Rng,
) -> Personality {
    let tables = &data.personality;
    let first = rng.index(tables.traits.len());
    let others: Vec<usize> = (0..tables.traits.len())
        .filter(|&i| {
            tables.traits.get(i).map(|t| &t.axis) != tables.traits.get(first).map(|t| &t.axis)
        })
        .collect();
    let second = rng.pick(&others).copied().unwrap_or(first);
    let traits: Vec<&crate::data::content::TraitEntry> = [first, second]
        .iter()
        .filter_map(|&i| tables.traits.get(i))
        .collect();
    let ideal = choose_ideal(data, category, rng);
    let bonds: Vec<&crate::data::content::BondEntry> = tables
        .bonds
        .iter()
        .filter(|b| match b.needs.as_str() {
            "family" => ctx.kin.is_some(),
            "work" => ctx.workplace.is_some(),
            "friend" => ctx.friend.is_some(),
            "rival" => ctx.rival.is_some(),
            _ => true,
        })
        .collect();
    let bond = rng.pick(&bonds).map_or_else(String::new, |b| {
        b.text
            .replace("{settlement}", ctx.settlement)
            .replace("{workplace}", ctx.workplace.unwrap_or("the workshop"))
            .replace("{kin}", ctx.kin.unwrap_or("family"))
            .replace("{friend}", ctx.friend.unwrap_or("an old friend"))
            .replace("{rival}", ctx.rival.unwrap_or("a neighbour"))
    });
    let flaw = rng.pick(&tables.flaws);
    let mannerism = rng.pick(&tables.mannerisms).cloned().unwrap_or_default();
    let adjectives: Vec<&str> = traits.iter().map(|t| t.short.as_str()).collect();
    let summary = format!(
        "{} {} who {}, but {}.",
        capitalise(&adjectives.join(" and ")),
        title.to_lowercase(),
        ideal.map_or("keeps to themselves", |i| i.short.as_str()),
        flaw.map_or("has no great faults", |f| f.short.as_str()),
    );
    Personality {
        traits: traits.iter().map(|t| t.text.clone()).collect(),
        ideal: ideal.map_or_else(
            || Ideal {
                name: String::new(),
                text: String::new(),
                alignment: "neutral".into(),
                tags: Vec::new(),
            },
            |i| Ideal {
                name: i.name.clone(),
                text: i.text.clone(),
                alignment: i.alignment.clone(),
                tags: i.tags.clone(),
            },
        ),
        bond,
        flaw: flaw.map_or_else(String::new, |f| f.text.clone()),
        mannerism,
        summary,
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |f| {
        f.to_uppercase().collect::<String>() + chars.as_str()
    })
}
