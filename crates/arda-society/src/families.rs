//! The template families the code reads, and the slots each call site fills.
//! [`lint`] checks the embedded history tables against this list: every key
//! in a table is a family the code uses, every family is present and deep
//! enough, and every `{slot}` is one its call site provides.

use crate::tables::HistoryTable;
use std::collections::BTreeMap;

/// One template family.
#[derive(Debug, Clone, Copy)]
pub struct Family {
    /// Key in its table.
    pub key: &'static str,
    /// Slots the call site fills.
    pub slots: &'static [&'static str],
    /// Fewest templates the family may hold.
    pub min: usize,
}

const fn f(key: &'static str, slots: &'static [&'static str], min: usize) -> Family {
    Family { key, slots, min }
}

const FOUND: &[&str] = &["settlement", "site"];
const FOUND_FROM: &[&str] = &["settlement", "site", "parent"];
const REALM: &[&str] = &["realm", "settlement", "count"];
const WAR: &[&str] = &[
    "realm_a", "realm_b", "place", "years", "cause", "year", "war",
];
const PEACE: &[&str] = &[
    "realm_a", "realm_b", "place", "years", "cause", "year", "war", "winner", "loser", "gains",
];
const SHIFT: &[&str] = &[
    "realm_a",
    "realm_b",
    "place",
    "years",
    "cause",
    "year",
    "war",
    "winner",
    "loser",
    "gains",
    "settlement",
];
const DIS: &[&str] = &["settlement", "places", "year", "realm", "river"];
const PLAGUE: &[&str] = &["settlement", "places", "year", "realm", "river", "plague"];
const BACK: &[&str] = &[
    "settlement",
    "places",
    "year",
    "realm",
    "river",
    "cause",
    "cause_year",
    "years",
    "name",
];
const WAR_BACK: &[&str] = &["realm_a", "realm_b", "place", "year", "years", "name"];
const ACCESSION: &[&str] = &["ruler", "title", "realm", "house"];
const DYN: &[&str] = &["house", "realm", "year", "settlement", "old_house"];
const DYN_BACK: &[&str] = &["house", "realm", "year", "settlement", "old_house", "war"];
const RUIN: &[&str] = &["ruin", "kind", "settlement", "year", "realm"];
const RUIN_BACK: &[&str] = &[
    "ruin",
    "kind",
    "settlement",
    "year",
    "realm",
    "event",
    "event_year",
];
const L_BASE: &[&str] = &["settlement", "site", "era", "year"];
const L_FROM: &[&str] = &["settlement", "site", "era", "year", "parent"];
const L_WAR: &[&str] = &["settlement", "site", "era", "year", "former", "war"];
const L_EVENT: &[&str] = &["settlement", "site", "era", "year", "event"];
const L_DYN: &[&str] = &["settlement", "site", "era", "year", "house", "realm"];
const L_RUIN: &[&str] = &["settlement", "site", "era", "year", "ruin", "kind"];
const L_RUIN_EVENT: &[&str] = &["settlement", "site", "era", "year", "ruin", "kind", "event"];
const L_EXPORT: &[&str] = &["settlement", "site", "era", "good", "dest"];
const L_PLAIN: &[&str] = &["settlement", "site", "era"];

/// Event title families.
pub const TITLES: &[Family] = &[
    f("founding_root", FOUND, 3),
    f("founding_from", FOUND_FROM, 3),
    f("realm_formed", REALM, 3),
    f("peace", PEACE, 3),
    f("border_shift", SHIFT, 3),
    f("flood", DIS, 3),
    f("plague", PLAGUE, 3),
    f("fire", DIS, 3),
    f("famine", DIS, 3),
    f("mine_collapse", DIS, 3),
    f("dynasty_founding", DYN, 3),
    f("dynasty_usurpation", DYN, 3),
    f("dynasty_succession", DYN, 3),
    f("dynasty_conquest", DYN, 3),
    f("dynasty_election", DYN, 3),
    f("abandon_plague", RUIN, 3),
    f("abandon_flood", RUIN, 3),
    f("abandon_war", RUIN, 3),
    f("abandon_famine", RUIN, 3),
    f("abandon_fire", RUIN, 3),
    f("abandon_mine_collapse", RUIN, 3),
    f("abandon_well_failed", RUIN, 3),
    f("abandon_raiders", RUIN, 3),
    f("abandon_enclosure", RUIN, 3),
];

/// Event text families.
pub const EVENTS: &[Family] = &[
    f("founding_root", FOUND, 4),
    f("founding_from", FOUND_FROM, 4),
    f("realm_formed", REALM, 4),
    f("war", WAR, 4),
    f("peace_gains", PEACE, 4),
    f("peace_status_quo", PEACE, 4),
    f("border_shift", SHIFT, 4),
    f("flood", DIS, 4),
    f("plague", PLAGUE, 4),
    f("fire", DIS, 4),
    f("famine", DIS, 4),
    f("mine_collapse", DIS, 4),
    f("accession", ACCESSION, 4),
    f("dynasty_founding", DYN, 4),
    f("dynasty_usurpation", DYN, 4),
    f("dynasty_succession", DYN, 4),
    f("dynasty_conquest", DYN, 4),
    f("dynasty_election", DYN, 4),
    f("abandon_plague", RUIN, 4),
    f("abandon_flood", RUIN, 4),
    f("abandon_war", RUIN, 4),
    f("abandon_famine", RUIN, 4),
    f("abandon_fire", RUIN, 4),
    f("abandon_mine_collapse", RUIN, 4),
    f("abandon_well_failed", RUIN, 4),
    f("abandon_raiders", RUIN, 4),
    f("abandon_enclosure", RUIN, 4),
];

/// Settlement history-hook families.
pub const LORE: &[Family] = &[
    f("site_line", &["line"], 1),
    f("root", L_BASE, 4),
    f("from", L_FROM, 4),
    f("captured", L_WAR, 4),
    f("captured_recent", L_WAR, 4),
    f("plague", L_EVENT, 4),
    f("plague_small", L_EVENT, 4),
    f("flood", L_EVENT, 4),
    f("fire", L_EVENT, 4),
    f("famine", L_EVENT, 4),
    f("mine_collapse", L_EVENT, 4),
    f("dynasty", L_DYN, 4),
    f("ruin", L_RUIN, 4),
    f("ruin_caused", L_RUIN_EVENT, 4),
    f("export", L_EXPORT, 4),
    f("quiet", L_PLAIN, 4),
];

/// Backstory families, `<kind>_<cause_kind>`.
pub const BACKSTORY: &[Family] = &[
    f("plague_foul_wells", BACK, 2),
    f("plague_ship_borne", BACK, 2),
    f("plague_famine_weakened", BACK, 2),
    f("plague_war_camp", BACK, 2),
    f("plague_crowded_lanes", BACK, 2),
    f("famine_river_spate", BACK, 2),
    f("famine_war_levy", BACK, 2),
    f("famine_rain_failure", BACK, 2),
    f("famine_blight", BACK, 2),
    f("flood_river_spate", BACK, 2),
    f("flood_snowmelt", BACK, 2),
    f("flood_storm_surge", BACK, 2),
    f("fire_lamp_fire", BACK, 2),
    f("fire_bad_timber", BACK, 2),
    f("mine_collapse_bad_timber", BACK, 2),
    f("mine_collapse_flooded_gallery", BACK, 2),
    f("war_toll_quarrel", WAR_BACK, 2),
    f("war_border_claim", WAR_BACK, 2),
    f("border_shift", SHIFT, 2),
    f("dynasty_usurpation", DYN_BACK, 2),
    f("dynasty_succession", DYN_BACK, 2),
    f("dynasty_conquest", DYN_BACK, 2),
    f("dynasty_election", DYN_BACK, 2),
    f("ruin_plague", RUIN_BACK, 2),
    f("ruin_flood", RUIN_BACK, 2),
    f("ruin_war", RUIN_BACK, 2),
    f("ruin_famine", RUIN_BACK, 2),
    f("ruin_fire", RUIN_BACK, 2),
    f("ruin_mine_collapse", RUIN_BACK, 2),
    f("ruin_well_failed", RUIN_BACK, 2),
    f("ruin_raiders", RUIN_BACK, 2),
    f("ruin_enclosure", RUIN_BACK, 2),
];

/// Event-name families (`event_names`).
pub const NAMES: &[Family] = &[
    f("flood", DIS, 3),
    f("famine", DIS, 3),
    f("fire", DIS, 3),
    f("dynasty_usurpation", DYN, 3),
    f("dynasty_succession", DYN, 3),
    f("dynasty_conquest", DYN, 3),
    f("dynasty_election", DYN, 3),
];

/// Slots of the `war_names` patterns.
pub const WAR_NAME_SLOTS: &[&str] = &["realm_a", "realm_b", "place", "years", "cause", "year"];

/// The slot names used in `template`, or the reason its braces are broken.
fn slots_of(template: &str) -> Result<Vec<&str>, String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find(['{', '}']) {
        if rest[open..].starts_with('}') {
            return Err(format!("stray `}}` in `{template}`"));
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find(['{', '}']) else {
            return Err(format!("unclosed `{{` in `{template}`"));
        };
        if !after[close..].starts_with('}') {
            return Err(format!("nested `{{` in `{template}`"));
        }
        out.push(&after[..close]);
        rest = &after[close + 1..];
    }
    Ok(out)
}

fn check_list(what: &str, templates: &[String], slots: &[&str], min: usize) -> Result<(), String> {
    if templates.len() < min {
        return Err(format!(
            "{what}: {} templates, at least {min} wanted",
            templates.len()
        ));
    }
    for t in templates {
        for slot in slots_of(t)? {
            if !slots.contains(&slot) {
                return Err(format!("{what}: unknown slot `{{{slot}}}` in `{t}`"));
            }
        }
    }
    Ok(())
}

fn check_section(
    what: &str,
    table: &BTreeMap<String, Vec<String>>,
    families: &[Family],
) -> Result<(), String> {
    for key in table.keys() {
        if !families.iter().any(|f| f.key == key) {
            return Err(format!("{what}: `{key}` is not a family the code uses"));
        }
    }
    for fam in families {
        let list = table.get(fam.key).map_or(&[][..], Vec::as_slice);
        if list.is_empty() {
            return Err(format!("{what}: family `{}` is empty or missing", fam.key));
        }
        check_list(&format!("{what}.{}", fam.key), list, fam.slots, fam.min)?;
    }
    Ok(())
}

/// Checks the history tables against the families above.
///
/// # Errors
/// A description of the first problem found.
pub fn lint(h: &HistoryTable) -> Result<(), String> {
    check_section("titles", &h.titles, TITLES)?;
    check_section("events", &h.events, EVENTS)?;
    check_section("lore", &h.lore, LORE)?;
    check_section("backstory", &h.backstory, BACKSTORY)?;
    check_section("event_names", &h.event_names, NAMES)?;
    check_list("war_names", &h.war_names, WAR_NAME_SLOTS, 12)?;
    check_list("plague_names", &h.plague_names, &[], 16)?;
    for tier in ["hamlet", "village"] {
        let kinds = h.ruin_kinds.get(tier).map_or(&[][..], Vec::as_slice);
        check_list(&format!("ruin_kinds.{tier}"), kinds, &[], 6)?;
    }
    Ok(())
}
