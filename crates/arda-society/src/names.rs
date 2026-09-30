//! Original syllable-built names for houses, rulers, places and ruins.

use crate::rng::Stream;
use crate::tables::{CultureNames, NameTable};
use crate::text::capitalise_first;

fn culture<'a>(t: &'a NameTable, key: &str) -> Option<&'a CultureNames> {
    t.cultures.get(key).or_else(|| t.cultures.get(&t.default))
}

fn stem(c: &CultureNames, rng: &mut Stream, syllables: u64, coda: bool) -> String {
    let mut out = String::new();
    for i in 0..syllables {
        if let Some(o) = rng.pick(&c.onsets) {
            out.push_str(o);
        }
        if let Some(n) = rng.pick(&c.nuclei) {
            out.push_str(n);
        }
        if coda && i + 1 == syllables {
            if let Some(k) = rng.pick(&c.codas) {
                out.push_str(k);
            }
        }
    }
    out
}

fn starts_consonant(s: &str) -> bool {
    s.chars().next().is_some_and(|c| !"aeiouy".contains(c))
}

fn build(
    t: &NameTable,
    key: &str,
    rng: &mut Stream,
    ends: fn(&CultureNames) -> &[String],
) -> String {
    let Some(c) = culture(t, key) else {
        return "Nameless".to_string();
    };
    // One open syllable plus an ending, sometimes two; a closing consonant
    // only before a vowel ending, so clusters stay pronounceable.
    let end = rng.pick(ends(c)).cloned().unwrap_or_default();
    let syl = if rng.chance(250) { 2 } else { 1 };
    let mut s = stem(c, rng, syl, false);
    if end.is_empty() || !starts_consonant(&end) {
        if let Some(k) = rng.pick(&c.codas) {
            s.push_str(k);
        }
    }
    s.push_str(&end);
    capitalise_first(&s)
}

/// A house or family name.
#[must_use]
pub fn house(t: &NameTable, culture: &str, rng: &mut Stream) -> String {
    build(t, culture, rng, |c| &c.house)
}

/// A given name; `female` picks the ending set.
#[must_use]
pub fn given(t: &NameTable, culture: &str, female: bool, rng: &mut Stream) -> String {
    if female {
        build(t, culture, rng, |c| &c.given_f)
    } else {
        build(t, culture, rng, |c| &c.given_m)
    }
}

/// A place name.
#[must_use]
pub fn place(t: &NameTable, culture: &str, rng: &mut Stream) -> String {
    build(t, culture, rng, |c| &c.place)
}
