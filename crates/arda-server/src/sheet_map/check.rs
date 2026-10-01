//! Startup validation of a mapping file, before any request: the shape of
//! every rule, pointer, template and conversion. The dry run over probe NPCs
//! lives in [`super::SheetMapping::from_json`].

use super::convert::Tables;
use super::pointer::{target, tokens};
use super::spec::{MappingFile, Op, Rule, FORMAT, VERSION};

/// Splits a template into literal text and `{/pointer}` placeholders.
///
/// # Errors
/// An unclosed `{`, a lone `}`, or a placeholder that is not a pointer.
pub fn template_parts(t: &str) -> Result<Vec<(bool, String)>, String> {
    let mut parts = Vec::new();
    let mut lit = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                lit.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                lit.push('}');
            }
            '{' => {
                let mut ptr = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(x) => ptr.push(x),
                        None => return Err(format!("template {t:?} has an unclosed '{{'")),
                    }
                }
                tokens(&ptr).map_err(|e| format!("template {t:?}: {e}"))?;
                if ptr.is_empty() {
                    return Err(format!("template {t:?} has an empty placeholder"));
                }
                parts.push((false, std::mem::take(&mut lit)));
                parts.push((true, ptr));
            }
            '}' => return Err(format!("template {t:?} has a lone '}}' (write '}}}}')")),
            x => lit.push(x),
        }
    }
    parts.push((false, lit));
    Ok(parts)
}

fn ops(list: &[Op], tables: &Tables, at: &str) -> Result<(), String> {
    for (i, op) in list.iter().enumerate() {
        let at = format!("{at}.convert[{i}]");
        let need = |name: &str| {
            tables
                .get(name)
                .map(|_| ())
                .ok_or_else(|| format!("{at}: no table {name:?} in `tables`"))
        };
        match op {
            Op::Table { table, .. } => need(table)?,
            Op::Keys { table, .. } => {
                need(table)?;
                if let Some((k, v)) = tables
                    .get(table)
                    .into_iter()
                    .flatten()
                    .find(|(_, v)| !v.is_string())
                {
                    return Err(format!(
                        "{at}: key table {table:?} maps {k:?} to {v}, not a string"
                    ));
                }
            }
            Op::Scale {
                factor,
                offset,
                round,
            } => {
                if !factor.is_finite() || !offset.is_finite() {
                    return Err(format!("{at}: factor and offset must be finite"));
                }
                if round.is_some_and(|r| r > 12) {
                    return Err(format!("{at}: round keeps at most 12 decimals"));
                }
            }
            Op::Replace { find, .. } if find.is_empty() => {
                return Err(format!("{at}: replace needs a non-empty `find`"))
            }
            Op::Split { sep } if sep.is_empty() => {
                return Err(format!("{at}: split needs a non-empty `sep`"))
            }
            Op::Each { convert } => ops(convert, tables, &at)?,
            _ => {}
        }
    }
    Ok(())
}

fn rule(r: &Rule, tables: &Tables, at: &str) -> Result<(), String> {
    target(&r.to).map_err(|e| format!("{at}.to: {e}"))?;
    let sources = usize::from(r.from.is_some())
        + usize::from(r.constant.is_some())
        + usize::from(r.template.is_some());
    if sources != 1 {
        return Err(format!(
            "{at}: give exactly one of `from`, `const` and `template`"
        ));
    }
    if let Some(f) = &r.from {
        tokens(f).map_err(|e| format!("{at}.from: {e}"))?;
    }
    if let Some(t) = &r.template {
        template_parts(t).map_err(|e| format!("{at}: {e}"))?;
    }
    if r.fields.is_some() && r.from.is_none() {
        return Err(format!("{at}: `fields` needs `from`"));
    }
    if let Some(filter) = &r.filter {
        if r.fields.is_none() {
            return Err(format!("{at}: `where` needs `fields`"));
        }
        for k in filter.keys() {
            tokens(k).map_err(|e| format!("{at}.where: {e}"))?;
        }
    }
    if r.optional && r.from.is_none() {
        return Err(format!("{at}: `optional` needs `from`"));
    }
    if r.default.is_some() && !r.optional {
        return Err(format!("{at}: `default` needs `optional: true`"));
    }
    if let Some(sub) = &r.fields {
        if sub.is_empty() {
            return Err(format!("{at}: `fields` is empty"));
        }
        for (i, s) in sub.iter().enumerate() {
            rule(s, tables, &format!("{at}.fields[{i}]"))?;
        }
    }
    ops(&r.convert, tables, at)
}

/// Checks everything that can be checked without an NPC.
///
/// # Errors
/// The first problem, naming its rule (`rules[3].fields[1].convert[0]`).
pub fn file(m: &MappingFile) -> Result<(), String> {
    if m.format != FORMAT {
        return Err(format!("format must be {FORMAT:?}, not {:?}", m.format));
    }
    if m.version != VERSION {
        return Err(format!(
            "version {} is not supported (this server reads {VERSION})",
            m.version
        ));
    }
    let name_ok = (1..=64).contains(&m.name.len())
        && m.name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
    if !name_ok {
        return Err(format!(
            "name {:?} must be 1-64 characters of A-Z, a-z, 0-9, '.', '_' or '-'",
            m.name
        ));
    }
    for (i, r) in m.rules.iter().enumerate() {
        rule(r, &m.tables, &format!("rules[{i}]"))?;
    }
    for (i, d) in m.drop.iter().enumerate() {
        target(d).map_err(|e| format!("drop[{i}]: {e}"))?;
    }
    Ok(())
}
