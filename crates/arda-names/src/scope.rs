//! Uniqueness within a scope (a realm, a map, a world).
//!
//! Rule: two places in one scope never share a name or a near-identical
//! spelling (case, apostrophes and doubled letters ignored). A clash draws
//! the name again; after enough clashes a distinguishing adjective is added
//! ("Upper Oakford"), as real neighbouring places do. A kind with few
//! plain names (streams draw from a few dozen) is then named after a
//! person ("Haldor's Brook"), and a name that still clashes takes the
//! lowest free Roman numeral ("Oakmark II"), so a scope never runs out
//! (logic/15 §name-unique; OI-49).

use crate::compose::{compound, join, Modifier};
use crate::language::Language;
use crate::person::founder;
use crate::place::{attempt, distinguished, kind_head, place_name, DISTINGUISH};
use crate::rng::{hash_str, Rng};
use crate::types::{Name, Part, PlaceSpec};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Fresh draws before distinguishing adjectives are tried (the later ones
/// name settlements after founders, see `FOUNDER_SALT`).
const REDRAWS: u64 = 48;
/// Draws of a name after a person ("Haldor's Brook") before numbering.
const FOUNDED: u64 = 64;

/// The set of names already used in a scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameScope {
    used: BTreeSet<String>,
}

/// Spelling skeleton used to detect clashes.
#[must_use]
pub fn skeleton(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
    {
        if !out.ends_with(c) {
            out.push(c);
        }
    }
    out
}

impl NameScope {
    /// An empty scope.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a name (or a near-identical spelling) is taken.
    #[must_use]
    pub fn contains(&self, native: &str) -> bool {
        self.used.contains(&skeleton(native))
    }

    /// Marks a name taken; false if it already was.
    pub fn reserve(&mut self, native: &str) -> bool {
        self.used.insert(skeleton(native))
    }

    /// Names taken.
    #[must_use]
    pub fn len(&self) -> usize {
        self.used.len()
    }

    /// Nothing taken yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.used.is_empty()
    }

    /// Names a place uniquely in this scope and reserves the name. The
    /// result depends on what the scope already holds, so name places in a
    /// stable order (for example by id).
    ///
    /// Never fails: past the plain draws, the distinguishing adjectives and
    /// the names after people, the plain name takes the lowest Roman
    /// numeral not yet given to it in this scope.
    pub fn place_name(&mut self, lang: &Language, spec: &PlaceSpec) -> Name {
        let mut last = None;
        for salt in 0..REDRAWS {
            if let Some(n) = attempt(lang, spec, salt) {
                if self.reserve(&n.native) {
                    return n;
                }
                last = Some(n);
            }
        }
        let base = last.unwrap_or_else(|| place_name(lang, spec));
        for &adj in DISTINGUISH {
            if let Some(n) = distinguished(lang, &base, adj) {
                if self.reserve(&n.native) {
                    return n;
                }
            }
        }
        for salt in REDRAWS..REDRAWS * 20 {
            if let Some(n) = attempt(lang, spec, salt) {
                if self.reserve(&n.native) {
                    return n;
                }
            }
        }
        for salt in 0..FOUNDED {
            if let Some(n) = founded(lang, spec, salt) {
                if self.reserve(&n.native) {
                    return n;
                }
            }
        }
        self.numbered(&base)
    }

    /// `base` with the lowest Roman numeral from II on that this scope has
    /// not given it. Numbered names are kept apart by their numeral (the
    /// spelling skeleton folds "II" into "I"), so this always ends.
    fn numbered(&mut self, base: &Name) -> Name {
        let stem = skeleton(&base.native);
        let mut k = 2_u64;
        loop {
            let numeral = roman(k);
            let native = format!("{} {numeral}", base.native);
            if !self.contains(&native) && self.used.insert(format!("{stem}#{k}")) {
                return Name {
                    native,
                    gloss: format!("{} {numeral}", base.gloss),
                    literal: format!("{} {numeral}", base.literal),
                    pronunciation: format!("{} {numeral}", base.pronunciation),
                    word: base.word.clone(),
                    parts: base.parts.clone(),
                };
            }
            k += 1;
        }
    }
}

/// A place named after a person: a founder's name before the kind's head
/// ("Haldor's Brook"), drawn from its own stream of the key.
fn founded(lang: &Language, spec: &PlaceSpec, salt: u64) -> Option<Name> {
    let mut rng = Rng::new(
        lang.seed,
        &[
            hash_str("place-founded"),
            lang.preset as u64,
            spec.key,
            spec.kind as u64,
            salt,
        ],
    );
    let (mut parts, mut gloss) = founder(lang, &mut rng);
    if salt % 2 == 1 {
        // Founders' names alone run to a few hundred per language; a
        // second element ("Gekyukin") multiplies them.
        parts.extend(founder(lang, &mut rng).0);
        let ph = join(&lang.phonology, &parts);
        gloss = lang.spell(&lang.speak(&ph));
        parts = vec![Part {
            meaning: None,
            gloss: gloss.clone(),
            ph,
        }];
    }
    compound(
        lang,
        &Modifier::Proper(parts, gloss),
        kind_head(spec.kind),
        false,
    )
}

/// Roman numeral of `k` (thousands as repeated `M`).
fn roman(mut k: u64) -> String {
    const DIGITS: [(u64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in DIGITS {
        while k >= v {
            out.push_str(s);
            k -= v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_ignores_case_and_doubles() {
        assert_eq!(skeleton("Tobbo's Hill"), "toboshil");
        assert_eq!(skeleton("TOBO"), skeleton("tobbo"));
    }

    #[test]
    fn roman_numerals() {
        let got: Vec<String> = [2, 4, 9, 14, 40, 90, 1994, 3000].map(roman).to_vec();
        assert_eq!(got, ["II", "IV", "IX", "XIV", "XL", "XC", "MCMXCIV", "MMM"]);
    }
}
