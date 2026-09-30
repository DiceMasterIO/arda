//! Prints each preset's lexicon (debug aid).
use arda_names::{meaning::ALL, Language, Preset};
fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2026_u64);
    for p in Preset::ALL {
        let l = Language::new(seed, p);
        let words: Vec<String> = ALL
            .iter()
            .take(60)
            .map(|&m| format!("{}={}", m.info().key, l.word(m)))
            .collect();
        println!(
            "{}: head_first={} linker={}\n  {}",
            p.key(),
            l.grammar.head_first,
            l.grammar.linker,
            words.join(" ")
        );
    }
}
