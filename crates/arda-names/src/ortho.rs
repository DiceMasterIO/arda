//! Romanisation: phonemes to Latin letters, per language.
//!
//! Rules: each phoneme has a spelling (table default, overridable per
//! language); a language may spell /k/ differently before front vowels
//! ("c" but "k" in "ke"), and some phonemes differently at the end of a
//! word ("-y" for /i/). Output is lowercase ASCII plus the apostrophe.

use crate::phoneme::{Ph, TABLE};
use serde::{Deserialize, Serialize};

/// A language's spelling rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ortho {
    /// Spelling per phoneme (index = phoneme).
    pub spell: Vec<String>,
    /// Word-final spellings.
    pub final_spell: Vec<(Ph, String)>,
    /// Spelling of /k/ before a front vowel, when it differs.
    pub k_front: Option<String>,
}

fn pairs(s: &str) -> Vec<(Ph, String)> {
    s.split_whitespace()
        .filter_map(|t| {
            let (code, spell) = t.split_once('=')?;
            Some((Ph::from_code(code)?, spell.to_string()))
        })
        .collect()
}

impl Ortho {
    /// Builds from preset override strings.
    #[must_use]
    pub fn new(overrides: &str, finals: &str, k_front: &str) -> Self {
        let mut spell: Vec<String> = TABLE.iter().map(|i| i.spell.to_string()).collect();
        for (p, s) in pairs(overrides) {
            if let Some(slot) = spell.get_mut(usize::from(p.0)) {
                *slot = s;
            }
        }
        Self {
            spell,
            final_spell: pairs(finals),
            k_front: (!k_front.is_empty()).then(|| k_front.to_string()),
        }
    }

    fn one(&self, w: &[Ph], i: usize) -> &str {
        let p = w[i];
        if i + 1 == w.len() {
            if let Some((_, s)) = self.final_spell.iter().find(|(f, _)| *f == p) {
                return s;
            }
        }
        if p == Ph::code("k") && w.get(i + 1).is_some_and(|n| n.is_front()) {
            if let Some(k) = &self.k_front {
                return k;
            }
        }
        self.spell.get(usize::from(p.0)).map_or("", String::as_str)
    }

    /// Lowercase spelling of a word.
    #[must_use]
    pub fn render(&self, w: &[Ph]) -> String {
        let mut out = String::with_capacity(w.len() * 2);
        for i in 0..w.len() {
            if i == 0 && w[i] == Ph::code("'") {
                continue;
            }
            out.push_str(self.one(w, i));
            // Never three identical letters in a row ("aaa").
            let b = out.as_bytes();
            let n = b.len();
            if n >= 3 && b[n - 1] == b[n - 2] && b[n - 2] == b[n - 3] {
                out.pop();
            }
        }
        out
    }

    /// Syllables joined by hyphens, the stressed one in capitals.
    #[must_use]
    pub fn respell(&self, w: &[Ph], starts: &[usize], stressed: usize) -> String {
        let mut parts = Vec::with_capacity(starts.len());
        for (n, &s) in starts.iter().enumerate() {
            let e = starts.get(n + 1).copied().unwrap_or(w.len());
            let mut part = String::new();
            for i in s..e {
                part.push_str(self.one(w, i));
            }
            let part = part.replace('\'', "");
            parts.push(if n == stressed {
                part.to_ascii_uppercase()
            } else {
                part
            });
        }
        parts.join("-")
    }
}

/// Capitalises the first letter of each space-separated word.
#[must_use]
pub fn capitalise(s: &str) -> String {
    s.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |f| {
                f.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(codes: &str) -> Vec<Ph> {
        codes.split(' ').map(Ph::code).collect()
    }

    #[test]
    fn spells_with_overrides_and_context() {
        let o = Ortho::new("k=c x=ch", "i=y", "k");
        assert_eq!(o.render(&w("k a x")), "cach");
        assert_eq!(o.render(&w("k e l i")), "kely");
        assert_eq!(capitalise("dun an ruath"), "Dun An Ruath");
        assert_eq!(o.respell(&w("k a l m a r"), &[0, 3], 0), "CAL-mar");
    }
}
