//! Review round 2 #42: a language's roots are never blocked or repeated,
//! even when the blocklist rules out most short words, and a place name's
//! last-resort fallback is never blocked either.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::filter::Blocklist;
use arda_names::language::{Language, Options};
use arda_names::preset::Preset;
use std::collections::BTreeSet;

#[test]
fn roots_stay_unblocked_and_distinct_under_a_harsh_blocklist() {
    let mut block = Blocklist::builtin().clone();
    for frag in ["a", "e", "i"] {
        block.add_fragment(frag);
    }
    for (seed, preset) in [
        (1, Preset::Heartland),
        (7, Preset::Highland),
        (9, Preset::Southern),
    ] {
        let opts = Options {
            substrate_seed: None,
            blocklist: Some(block.clone()),
        };
        let lang = Language::with_options(seed, preset, &opts);
        let mut seen = BTreeSet::new();
        for root in lang.lexicon.iter().chain(&lang.name_roots) {
            let s = lang.ortho.render(root);
            assert!(!block.blocks(&s), "{preset:?}: root {s:?} is blocked");
            assert!(seen.insert(s.clone()), "{preset:?}: root {s:?} repeats");
        }
    }
}
