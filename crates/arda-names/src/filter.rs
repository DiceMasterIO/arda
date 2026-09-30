//! Profanity and real-word filter over an original, extendable blocklist.
//!
//! Rule: a name is rejected when any of its words equals a blocked word, or
//! when the whole name contains a blocked fragment. Generators then draw
//! again, so a blocked word never reaches output.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::OnceLock;

const BUILTIN: &str = include_str!("blocklist.txt");

/// Words and fragments no generated name may use.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocklist {
    words: BTreeSet<String>,
    fragments: BTreeSet<String>,
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

impl Blocklist {
    /// The list shipped with the crate (`src/blocklist.txt`).
    #[must_use]
    pub fn builtin() -> &'static Blocklist {
        static LIST: OnceLock<Blocklist> = OnceLock::new();
        LIST.get_or_init(|| {
            let mut b = Blocklist::default();
            b.extend_from_str(BUILTIN);
            b
        })
    }

    /// Adds entries in the blocklist file format.
    pub fn extend_from_str(&mut self, text: &str) {
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            match line.strip_prefix('*') {
                Some(frag) => self.add_fragment(frag),
                None => self.add_word(line),
            }
        }
    }

    /// Blocks a whole word.
    pub fn add_word(&mut self, word: &str) {
        let w = norm(word);
        if !w.is_empty() {
            self.words.insert(w);
        }
    }

    /// Blocks a fragment anywhere.
    pub fn add_fragment(&mut self, frag: &str) {
        let f = norm(frag);
        if !f.is_empty() {
            self.fragments.insert(f);
        }
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.words.len() + self.fragments.len()
    }

    /// No entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether `name` is blocked.
    #[must_use]
    pub fn blocks(&self, name: &str) -> bool {
        if name
            .split([' ', '-'])
            .any(|w| self.words.contains(&norm(w)))
        {
            return true;
        }
        let whole = norm(name);
        self.fragments.iter().any(|f| whole.contains(f.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_blocks_words_and_fragments() {
        let b = Blocklist::builtin();
        assert!(b.len() > 50);
        assert!(b.blocks("Tit"));
        assert!(!b.blocks("Titharen"));
        assert!(b.blocks("Mashitor"));
        assert!(b.blocks("Dun Mordor"));
        assert!(!b.blocks("Kelmarton"));
    }

    #[test]
    fn maintainer_can_extend() {
        let mut b = Blocklist::builtin().clone();
        assert!(!b.blocks("Kelmar"));
        b.extend_from_str("# local\nkelmar\n*zzq\n");
        assert!(b.blocks("kelmar"));
        assert!(b.blocks("Buzzqa"));
    }
}
