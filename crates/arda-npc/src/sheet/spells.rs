//! A small, deterministic selection of SRD spells for a class build.

use crate::data::srd::ClassData;
use crate::data::Data;
use crate::rng::Rng;
use crate::sheet::SpellRef;

/// Spells picked per available spell level.
const SPELLS_PER_LEVEL: usize = 2;

/// Cantrips known at a class level (SRD columns at levels 1, 4 and 10).
pub(crate) fn cantrips_known(class: &ClassData, level: u8) -> u8 {
    let [early, mid, late] = class.cantrips_known;
    match level {
        0..=3 => early,
        4..=9 => mid,
        _ => late,
    }
}

/// Cantrips plus up to two spells of every level the slots reach.
pub(crate) fn choose_spells(
    data: &Data,
    class: &ClassData,
    level: u8,
    slots: &[u8],
    rng: &mut Rng,
) -> Vec<SpellRef> {
    let mut chosen = Vec::new();
    let pick = |spell_level: u8, count: usize, rng: &mut Rng, chosen: &mut Vec<SpellRef>| {
        let mut pool: Vec<&str> = data
            .srd
            .spells
            .iter()
            .filter(|s| s.level == spell_level && s.classes.contains(&class.name))
            .map(|s| s.name.as_str())
            .collect();
        // Signature cantrip first where the class has one.
        if spell_level == 0 {
            if let Some(i) = pool.iter().position(|&n| n == "Eldritch Blast") {
                let blast = pool.remove(i);
                chosen.push(SpellRef {
                    name: blast.to_string(),
                    level: 0,
                });
            }
        }
        rng.shuffle(&mut pool);
        let already = chosen.iter().filter(|s| s.level == spell_level).count();
        for name in pool.into_iter().take(count.saturating_sub(already)) {
            chosen.push(SpellRef {
                name: name.to_string(),
                level: spell_level,
            });
        }
    };
    let cantrips = usize::from(cantrips_known(class, level));
    if cantrips > 0 {
        pick(0, cantrips, rng, &mut chosen);
    }
    for spell_level in 1..=slots.len() {
        let Ok(spell_level) = u8::try_from(spell_level) else {
            break;
        };
        pick(spell_level, SPELLS_PER_LEVEL, rng, &mut chosen);
    }
    chosen.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.name.cmp(&b.name)));
    chosen
}
