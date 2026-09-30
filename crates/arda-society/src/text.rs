//! Template filling for generated prose. Templates use `{key}` slots.

use crate::rng::Stream;
use std::collections::BTreeMap;

/// Slot values for one template.
#[derive(Debug, Clone, Default)]
pub struct Slots(BTreeMap<&'static str, String>);

impl Slots {
    /// Empty slot set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces a slot, builder style.
    #[must_use]
    pub fn with(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.0.insert(key, value.into());
        self
    }

    /// Whether `key` is set.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Adds or replaces a slot in place.
    pub fn set(&mut self, key: &'static str, value: impl Into<String>) {
        self.0.insert(key, value.into());
    }

    /// Fills `template` and capitalises the first letter (for sentences).
    #[must_use]
    pub fn fill(&self, template: &str) -> String {
        capitalise_sentences(&self.fill_name(template))
    }

    /// Fills `template` as written (for names used mid-sentence).
    /// Unknown slots are left in place so tests catch them.
    #[must_use]
    pub fn fill_name(&self, template: &str) -> String {
        let mut out = String::with_capacity(template.len() + 32);
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let Some(close) = after.find('}') else {
                out.push_str(&rest[open..]);
                return out;
            };
            let key = &after[..close];
            match self.0.get(key) {
                Some(v) => out.push_str(v),
                None => {
                    out.push('{');
                    out.push_str(key);
                    out.push('}');
                }
            }
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        out
    }
}

/// Picks one name template and fills it without capitalising.
#[must_use]
pub fn pick_name(options: &[String], rng: &mut Stream, slots: &Slots) -> String {
    rng.pick(options)
        .map_or_else(String::new, |t| slots.fill_name(t))
}

/// Title-cases every word ("salt fish" → "Salt Fish").
#[must_use]
pub fn title_case(s: &str) -> String {
    s.split(' ')
        .map(capitalise_first)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Picks one template from `options` with `rng` and fills it.
#[must_use]
pub fn pick_fill(options: &[String], rng: &mut Stream, slots: &Slots) -> String {
    rng.pick(options)
        .map_or_else(String::new, |t| slots.fill(t))
}

/// A mid-sentence phrase for an event: "the Grey Cough", "the flood of 212".
#[must_use]
pub fn event_phrase(e: &crate::history::Event) -> String {
    use crate::history::EventKind;
    match e.kind {
        EventKind::Plague => e
            .title
            .strip_prefix("The ")
            .map_or_else(|| e.title.clone(), |r| format!("the {r}")),
        EventKind::Flood => format!("the flood of {}", e.year),
        EventKind::Fire => format!("the great fire of {}", e.year),
        EventKind::Famine => format!("the hungry years of {}", e.year),
        EventKind::MineCollapse => format!("the mine collapse of {}", e.year),
        _ => e.title.clone(),
    }
}

/// "one road", "two roads", "5 roads".
#[must_use]
pub fn count(n: usize, one: &str, many: &str) -> String {
    const WORDS: [&str; 10] = [
        "no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    let word = WORDS
        .get(n)
        .map_or_else(|| n.to_string(), |w| (*w).to_string());
    format!("{word} {}", if n == 1 { one } else { many })
}

/// Upper-cases the first letter of the text and of every sentence after
/// `. `, `? ` or `! `.
#[must_use]
pub fn capitalise_sentences(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut up = true;
    let mut prev_stop = false;
    for ch in s.chars() {
        if up && ch.is_alphabetic() {
            out.extend(ch.to_uppercase());
            up = false;
        } else {
            out.push(ch);
            if ch.is_alphanumeric() {
                up = false;
            }
        }
        if ch == ' ' && prev_stop {
            up = true;
        }
        prev_stop = matches!(ch, '.' | '?' | '!');
    }
    out
}

/// Upper-cases the first letter.
#[must_use]
pub fn capitalise_first(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

/// "a, b and c".
#[must_use]
pub fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// Roman numeral for a regnal number ("II", "XIV"), capped at 49.
#[must_use]
pub fn roman(n: u32) -> String {
    const PAIRS: [(u32, &str); 6] = [
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut n = n.min(49);
    let mut out = String::new();
    for (v, s) in PAIRS {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_replaces_known_slots_only() {
        let s = Slots::new().with("a", "one").with("b", "two");
        assert_eq!(s.fill("{a} and {b}, {c}"), "One and two, {c}");
        assert_eq!(
            join_and(&["x".into(), "y".into(), "z".into()]),
            "x, y and z"
        );
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(14), "XIV");
    }
}
