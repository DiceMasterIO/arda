//! Composition: morphemes to compounds, with English glosses.
//!
//! Rules:
//! - A place name is modifier + head in head-last languages and head +
//!   (linker) + modifier in head-first ones; the head comes from the site
//!   (a ford gives "ford", an estuary "mouth") or else the settlement kind.
//! - Countable modifiers may take the plural ("ford of the oaks").
//! - Morphemes meet through sandhi (see [`crate::sandhi`]), affix vowels
//!   follow vowel harmony, and names over four syllables lose a medial
//!   unstressed vowel when that stays legal.
//! - The English gloss is always built head-last, as English builds place
//!   names ("Oakford", "Kingsbridge").

use crate::harmony::{last_class, to_class};
use crate::language::Language;
use crate::meaning::{Class, Meaning};
use crate::phonology::Phonology;
use crate::sandhi::repair;
use crate::types::{Name, Part};

/// Longest name, in syllables, before syncope is tried.
const MAX_SYLLABLES: usize = 4;

/// A morpheme of `lang` for `m`.
pub(crate) fn part(lang: &Language, m: Meaning) -> Part {
    Part {
        meaning: Some(m),
        gloss: m.info().modifier.to_string(),
        ph: lang.root(m).to_vec(),
    }
}

fn is_affix(p: &Part) -> bool {
    p.meaning.is_some_and(|m| m.info().class == Class::Affix)
}

/// Joins morphemes into one legal word.
pub(crate) fn join(ph: &Phonology, parts: &[Part]) -> Vec<Ph> {
    let mut w: Vec<Ph> = Vec::with_capacity(12);
    for p in parts {
        let front = if ph.harmony && is_affix(p) {
            last_class(&w)
        } else {
            None
        };
        match front {
            Some(f) => w.extend(p.ph.iter().map(|&v| to_class(v, f))),
            None => w.extend_from_slice(&p.ph),
        }
    }
    repair(ph, &mut w);
    shorten(ph, &mut w);
    w
}

use crate::phoneme::Ph;

/// Syncope: over-long words lose the vowel of an unstressed medial
/// syllable (the one nearest after the stress first), then are repaired.
fn shorten(ph: &Phonology, w: &mut Vec<Ph>) {
    for _ in 0..2 {
        let starts = ph.syllables(w);
        let n = starts.len();
        if n <= MAX_SYLLABLES {
            return;
        }
        let stressed = ph.stressed(n);
        let mut order: Vec<usize> = (1..n - 1).filter(|&s| s != stressed).collect();
        order.sort_by_key(|&s| (s < stressed, s.abs_diff(stressed)));
        let shorter = order.into_iter().find_map(|t| {
            let (s, e) = (starts[t], starts.get(t + 1).copied().unwrap_or(w.len()));
            let v = (s..e).find(|&i| w[i].is_vowel())?;
            let mut x = w.clone();
            x.remove(v);
            repair(ph, &mut x);
            (ph.valid(&x) && ph.syllables(&x).len() < n).then_some(x)
        });
        match shorter {
            Some(x) => *w = x,
            None => return,
        }
    }
}

/// Speaks, spells and filters a finished word; `None` when blocked.
pub(crate) fn finish(
    lang: &Language,
    word: Vec<Ph>,
    parts: Vec<Part>,
    gloss: &str,
    literal: String,
) -> Option<Name> {
    let spoken = lang.speak(&word);
    let native = lang.spell(&spoken);
    if native.is_empty() || lang.blocklist.blocks(&native) {
        return None;
    }
    Some(Name {
        pronunciation: lang.pronounce(&spoken),
        native,
        gloss: tidy_gloss(gloss),
        literal,
        word,
        parts,
    })
}

/// Collapses triple letters in English glosses ("Bellley" → "Belley").
fn tidy_gloss(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let mut tail = out.chars().rev();
        let lc = c.to_ascii_lowercase();
        if tail.next().map(|x| x.to_ascii_lowercase()) == Some(lc)
            && tail.next().map(|x| x.to_ascii_lowercase()) == Some(lc)
        {
            continue;
        }
        out.push(c);
    }
    out
}

/// What fills the modifier slot.
#[derive(Debug, Clone)]
pub(crate) enum Modifier {
    /// A lexicon meaning.
    Meaning(Meaning),
    /// A proper name (a founder, a capital): its parts and English form.
    Proper(Vec<Part>, String),
}

/// English and literal glosses for a modifier + head compound.
fn glosses(modi: &Modifier, head: Meaning, plural: bool, head_gloss: &str) -> (String, String) {
    let hw = head.info().word;
    match modi {
        Modifier::Proper(_, name) => (format!("{name}{head_gloss}"), format!("{name}'s {hw}")),
        Modifier::Meaning(m) => {
            let i = m.info();
            let english = match i.class {
                Class::Being if !i.modifier.ends_with('s') => {
                    format!("{}s{head_gloss}", i.modifier)
                }
                _ => format!("{}{head_gloss}", i.modifier),
            };
            let literal = match i.class {
                Class::Adj => format!("{} {hw}", i.word),
                Class::Being if plural => format!("{hw} of the {}", i.plural),
                Class::Being => format!("the {}'s {hw}", i.word),
                _ if i.plural.is_empty() => format!("{} {hw}", i.word),
                _ if plural => format!("{hw} of the {}", i.plural),
                _ => format!("{} {hw}", i.word),
            };
            (english, literal)
        }
    }
}

/// Builds modifier + head in the language's order. A head-first language
/// links possessors and plurals with its genitive ("ford of the oaks");
/// a result over four syllables drops the plural and then the linker.
pub(crate) fn compound(
    lang: &Language,
    modi: &Modifier,
    head: Meaning,
    plural: bool,
) -> Option<Name> {
    let (mut plural, mut linked) = (plural, true);
    loop {
        let (word, parts) = build(lang, modi, head, plural, linked);
        let long = lang.phonology.syllables(&word).len() > MAX_SYLLABLES;
        if long && plural {
            plural = false;
            continue;
        }
        if long && linked && parts.iter().any(|p| p.meaning == Some(Meaning::Of)) {
            linked = false;
            continue;
        }
        let (gloss, literal) = glosses(modi, head, plural, head.info().head);
        return finish(lang, word, parts, &gloss, literal);
    }
}

fn build(
    lang: &Language,
    modi: &Modifier,
    head: Meaning,
    plural: bool,
    linked: bool,
) -> (Vec<Ph>, Vec<Part>) {
    let mut mod_parts = match modi {
        Modifier::Meaning(m) => vec![part(lang, *m)],
        Modifier::Proper(p, _) => p.clone(),
    };
    let possessor = match modi {
        Modifier::Meaning(m) => m.info().class == Class::Being,
        Modifier::Proper(..) => true,
    };
    if plural {
        mod_parts.push(part(lang, Meaning::Plural));
    }
    let head_part = part(lang, head);
    let mut parts = Vec::with_capacity(mod_parts.len() + 2);
    if lang.grammar.head_first {
        parts.push(head_part);
        if lang.grammar.linker && linked && (possessor || plural) {
            parts.push(part(lang, Meaning::Of));
        }
        parts.extend(mod_parts);
    } else {
        parts.extend(mod_parts);
        parts.push(head_part);
    }
    (join(&lang.phonology, &parts), parts)
}

/// A name from the substrate tongue, borrowed into `lang`: built there,
/// then adapted to this inventory and phonotactics and spoken with this
/// dialect's changes.
/// With `simplex`, the old word stands alone when it has two syllables or
/// more, as many real river names do.
pub(crate) fn borrowed(
    lang: &Language,
    modi: Meaning,
    head: Meaning,
    english_head: &str,
    simplex: bool,
) -> Option<Name> {
    let sub = lang.substrate.as_deref()?;
    let alone = simplex && sub.phonology.syllables(sub.root(modi)).len() >= 2;
    let sub_parts = if alone {
        vec![part(sub, modi)]
    } else {
        vec![part(sub, modi), part(sub, head)]
    };
    let mut word = join(&sub.phonology, &sub_parts);
    repair(&lang.phonology, &mut word);
    let i = modi.info();
    let gloss = format!("{}{english_head}", i.modifier);
    let literal = if alone {
        let one = if i.class == Class::Adj { " one" } else { "" };
        format!("the {}{one}, in the old tongue", i.word)
    } else {
        format!("{} {}, in the old tongue", i.word, head.info().word)
    };
    let parts = vec![Part {
        meaning: None,
        gloss: gloss.clone(),
        ph: word.clone(),
    }];
    finish(lang, word, parts, &gloss, literal)
}

/// Derives a name by adding a head to a whole existing name ("Oakford" →
/// "Oakfordshire").
pub(crate) fn derived(
    lang: &Language,
    base: &Name,
    head: Meaning,
    english_head: &str,
    literal: String,
) -> Option<Name> {
    let modi = Modifier::Proper(
        vec![Part {
            meaning: None,
            gloss: base.gloss.clone(),
            ph: base.word.clone(),
        }],
        base.gloss.clone(),
    );
    let mut name = compound(lang, &modi, head, false)?;
    name.gloss = tidy_gloss(&format!("{}{english_head}", base.gloss));
    name.literal = literal;
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gloss_tidies_triples() {
        assert_eq!(tidy_gloss("Bellley"), "Belley");
        assert_eq!(tidy_gloss("Oakford"), "Oakford");
    }

    #[test]
    fn glosses_follow_english_patterns() {
        let (g, l) = glosses(
            &Modifier::Meaning(Meaning::Oak),
            Meaning::Ford,
            true,
            "ford",
        );
        assert_eq!((g.as_str(), l.as_str()), ("Oakford", "ford of the oaks"));
        let (g, l) = glosses(
            &Modifier::Meaning(Meaning::King),
            Meaning::Bridge,
            false,
            "bridge",
        );
        assert_eq!(
            (g.as_str(), l.as_str()),
            ("Kingsbridge", "the king's bridge")
        );
        let (g, l) = glosses(
            &Modifier::Meaning(Meaning::White),
            Meaning::Hill,
            false,
            "hill",
        );
        assert_eq!((g.as_str(), l.as_str()), ("Whitehill", "white hill"));
    }
}
