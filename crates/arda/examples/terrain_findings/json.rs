//! Minimal deterministic JSON writer: insertion-ordered objects, fixed
//! float precision, no external crate.

use std::fmt::Write;

/// A JSON value.
pub enum J {
    /// Integer.
    I(i64),
    /// Float, printed with four decimals; non-finite becomes `null`.
    F(f64),
    /// Boolean.
    B(bool),
    /// String (escaped on output).
    S(String),
    /// Array.
    A(Vec<J>),
    /// Insertion-ordered object.
    O(Vec<(String, J)>),
}

impl J {
    /// An empty object.
    pub fn obj() -> Self {
        Self::O(Vec::new())
    }

    /// Adds a key to an object (no-op on other variants).
    #[must_use]
    pub fn with(mut self, key: &str, v: J) -> Self {
        if let Self::O(items) = &mut self {
            items.push((key.to_string(), v));
        }
        self
    }

    /// Integer array from counts.
    pub fn counts(v: &[u64]) -> Self {
        Self::A(
            v.iter()
                .map(|&c| Self::I(i64::try_from(c).unwrap_or(i64::MAX)))
                .collect(),
        )
    }

    /// Integer from `usize`.
    pub fn u(v: usize) -> Self {
        Self::I(i64::try_from(v).unwrap_or(i64::MAX))
    }

    /// Renders the value with two-space indentation.
    pub fn render(&self) -> String {
        let mut s = String::new();
        self.write(&mut s, 0);
        s.push('\n');
        s
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Self::I(v) => {
                let _ = write!(out, "{v}");
            }
            Self::F(v) if v.is_finite() => {
                let _ = write!(out, "{v:.4}");
            }
            Self::F(_) => out.push_str("null"),
            Self::B(v) => out.push_str(if *v { "true" } else { "false" }),
            Self::S(v) => {
                out.push('"');
                for c in v.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        c if (c as u32) < 0x20 => {
                            let _ = write!(out, "\\u{:04x}", c as u32);
                        }
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            Self::A(items) => {
                if items
                    .iter()
                    .all(|i| matches!(i, Self::I(_) | Self::F(_) | Self::B(_)))
                {
                    out.push('[');
                    for (n, i) in items.iter().enumerate() {
                        if n > 0 {
                            out.push(',');
                        }
                        i.write(out, depth + 1);
                    }
                    out.push(']');
                } else {
                    out.push_str("[\n");
                    for (n, i) in items.iter().enumerate() {
                        out.push_str(&"  ".repeat(depth + 1));
                        i.write(out, depth + 1);
                        out.push_str(if n + 1 < items.len() { ",\n" } else { "\n" });
                    }
                    out.push_str(&"  ".repeat(depth));
                    out.push(']');
                }
            }
            Self::O(items) => {
                out.push_str("{\n");
                for (n, (k, v)) in items.iter().enumerate() {
                    out.push_str(&"  ".repeat(depth + 1));
                    let _ = write!(out, "\"{k}\": ");
                    v.write(out, depth + 1);
                    out.push_str(if n + 1 < items.len() { ",\n" } else { "\n" });
                }
                out.push_str(&"  ".repeat(depth));
                out.push('}');
            }
        }
    }
}
