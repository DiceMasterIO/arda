//! Uniqueness within a scope (a realm, a map, a world).
//!
//! Rule: two places in one scope never share a name or a near-identical
//! spelling (case, apostrophes and doubled letters ignored). A clash draws
//! the name again; after enough clashes a distinguishing adjective is added
//! ("Upper Oakford"), as real neighbouring places do.

use crate::error::NamesError;
use crate::language::Language;
use crate::place::{attempt, distinguished, place_name, DISTINGUISH};
use crate::types::{Name, PlaceSpec};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Fresh draws before distinguishing adjectives are tried (the later ones
/// name settlements after founders, see `FOUNDER_SALT`).
const REDRAWS: u64 = 48;

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
    /// # Errors
    /// [`NamesError::ScopeExhausted`] when every candidate is taken: the
    /// scope refuses rather than hand out a duplicate.
    pub fn place_name(&mut self, lang: &Language, spec: &PlaceSpec) -> Result<Name, NamesError> {
        let mut last = None;
        for salt in 0..REDRAWS {
            if let Some(n) = attempt(lang, spec, salt) {
                if self.reserve(&n.native) {
                    return Ok(n);
                }
                last = Some(n);
            }
        }
        let base = last.unwrap_or_else(|| place_name(lang, spec));
        for &adj in DISTINGUISH {
            if let Some(n) = distinguished(lang, &base, adj) {
                if self.reserve(&n.native) {
                    return Ok(n);
                }
            }
        }
        for salt in REDRAWS..REDRAWS * 20 {
            if let Some(n) = attempt(lang, spec, salt) {
                if self.reserve(&n.native) {
                    return Ok(n);
                }
            }
        }
        Err(NamesError::ScopeExhausted {
            kind: format!("{:?}", spec.kind).to_lowercase(),
            key: spec.key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_ignores_case_and_doubles() {
        assert_eq!(skeleton("Tobbo's Hill"), "toboshil");
        assert_eq!(skeleton("TOBO"), skeleton("tobbo"));
    }
}
