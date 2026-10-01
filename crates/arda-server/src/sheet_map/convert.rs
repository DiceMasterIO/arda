//! Value conversions of the mapping layer (`convert` lists): enum tables,
//! key renames, unit scaling, formatting and casing. `null` passes through
//! every conversion unchanged.

use super::pointer::kind;
use super::spec::{Case, Op, Unknown};
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;

/// The named tables of a mapping.
pub type Tables = BTreeMap<String, BTreeMap<String, Value>>;

/// Applies `ops` to `v` in order.
///
/// # Errors
/// A conversion that does not fit the value, or a value a strict table lacks.
pub fn run(ops: &[Op], tables: &Tables, mut v: Value) -> Result<Value, String> {
    for op in ops {
        if v.is_null() {
            return Ok(v);
        }
        v = one(op, tables, v)?;
    }
    Ok(v)
}

fn table<'a>(tables: &'a Tables, name: &str) -> Result<&'a BTreeMap<String, Value>, String> {
    tables.get(name).ok_or_else(|| format!("no table {name:?}"))
}

/// A scalar's text: strings as they are, numbers and booleans as JSON.
fn text(v: &Value, what: &str) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Number(_) | Value::Bool(_) => Ok(v.to_string()),
        other => Err(format!("{what} needs a scalar, got a {}", kind(other))),
    }
}

fn string<'a>(v: &'a Value, what: &str) -> Result<&'a str, String> {
    v.as_str()
        .ok_or_else(|| format!("{what} needs a string, got a {}", kind(v)))
}

fn number(x: f64) -> Result<Value, String> {
    Number::from_f64(x)
        .map(Value::Number)
        .ok_or_else(|| format!("{x} is not a finite number"))
}

fn scale(v: &Value, factor: f64, offset: f64, round: Option<u8>) -> Result<Value, String> {
    let x = v
        .as_f64()
        .ok_or_else(|| format!("scale needs a number, got a {}", kind(v)))?;
    let mut y = x * factor + offset;
    match round {
        None => number(y),
        Some(0) => {
            y = y.round();
            // An integer-valued f64 printed without decimals is its integer.
            format!("{y:.0}")
                .parse::<i64>()
                .map(Value::from)
                .map_err(|_| format!("{y} does not fit an integer"))
        }
        Some(n) => {
            let p = 10_f64.powi(i32::from(n));
            number((y * p).round() / p)
        }
    }
}

fn recase(s: &str, to: Case) -> String {
    match to {
        Case::Lower => s.to_lowercase(),
        Case::Upper => s.to_uppercase(),
        Case::Title => {
            let mut out = String::with_capacity(s.len());
            let mut start = true;
            for c in s.chars() {
                if start {
                    out.extend(c.to_uppercase());
                } else {
                    out.push(c);
                }
                start = c.is_whitespace() || c == '-' || c == '_';
            }
            out
        }
        Case::Kebab | Case::Snake => {
            let sep = if to == Case::Kebab { '-' } else { '_' };
            let mut out = String::with_capacity(s.len());
            let mut pending = false;
            for c in s.chars() {
                if c == '\'' || c == '\u{2019}' {
                    continue;
                }
                if c.is_alphanumeric() {
                    if pending && !out.is_empty() {
                        out.push(sep);
                    }
                    pending = false;
                    out.extend(c.to_lowercase());
                } else {
                    pending = true;
                }
            }
            out
        }
    }
}

fn keys(v: Value, t: &BTreeMap<String, Value>, unknown: Unknown) -> Result<Value, String> {
    let Value::Object(map) = v else {
        return Err(format!("keys needs an object, got a {}", kind(&v)));
    };
    let mut out = Map::new();
    for (k, val) in map {
        let key = match (t.get(&k), unknown) {
            (Some(Value::String(new)), _) => new.clone(),
            (Some(other), _) => return Err(format!("key table value {other} is not a string")),
            (None, Unknown::Keep) => k,
            (None, Unknown::Null) => continue,
            (None, Unknown::Error) => return Err(format!("key {k:?} is not in the table")),
        };
        if out.insert(key.clone(), val).is_some() {
            return Err(format!("two keys map to {key:?}"));
        }
    }
    Ok(Value::Object(out))
}

fn one(op: &Op, tables: &Tables, v: Value) -> Result<Value, String> {
    match op {
        Op::Table {
            table: name,
            unknown,
        } => {
            let key = text(&v, "table")?;
            match (table(tables, name)?.get(&key), unknown) {
                (Some(hit), _) => Ok(hit.clone()),
                (None, Unknown::Keep) => Ok(v),
                (None, Unknown::Null) => Ok(Value::Null),
                (None, Unknown::Error) => Err(format!("{key:?} is not in table {name:?}")),
            }
        }
        Op::Keys {
            table: name,
            unknown,
        } => keys(v, table(tables, name)?, *unknown),
        Op::Scale {
            factor,
            offset,
            round,
        } => scale(&v, *factor, *offset, *round),
        Op::Format { pattern } => Ok(Value::String(pattern.replace("{}", &text(&v, "format")?))),
        Op::Case { to } => Ok(Value::String(recase(string(&v, "case")?, *to))),
        Op::Replace { find, with } => Ok(Value::String(string(&v, "replace")?.replace(find, with))),
        Op::Join { sep } => {
            let Value::Array(items) = &v else {
                return Err(format!("join needs an array, got a {}", kind(&v)));
            };
            let parts: Result<Vec<String>, String> =
                items.iter().map(|x| text(x, "join")).collect();
            Ok(Value::String(parts?.join(sep)))
        }
        Op::Split { sep } => {
            let s = string(&v, "split")?;
            Ok(Value::Array(if s.is_empty() {
                Vec::new()
            } else {
                s.split(sep.as_str()).map(Value::from).collect()
            }))
        }
        Op::ToString => Ok(Value::String(text(&v, "to_string")?)),
        Op::ToNumber => match &v {
            Value::Number(_) => Ok(v),
            Value::String(s) => match s.trim().parse::<i64>() {
                Ok(i) => Ok(Value::from(i)),
                Err(_) => s
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| format!("{s:?} is not a number"))
                    .and_then(number),
            },
            other => Err(format!("to_number needs a string, got a {}", kind(other))),
        },
        Op::Each { convert } => {
            let Value::Array(items) = v else {
                return Err(format!("each needs an array, got a {}", kind(&v)));
            };
            items
                .into_iter()
                .map(|x| run(convert, tables, x))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ops(v: Value) -> Vec<Op> {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn conversions_cover_units_enums_and_text() {
        let mut tables = Tables::new();
        tables.insert(
            "ab".into(),
            [("STR".to_owned(), json!("strength"))]
                .into_iter()
                .collect(),
        );
        let t = &tables;
        let r = |o: Value, v: Value| run(&ops(o), t, v);
        assert_eq!(
            r(json!([{"op":"table","table":"ab"}]), json!("STR")).unwrap(),
            json!("strength")
        );
        assert!(r(json!([{"op":"table","table":"ab"}]), json!("DEX")).is_err());
        assert_eq!(
            r(
                json!([{"op":"table","table":"ab","unknown":"keep"}]),
                json!("DEX")
            )
            .unwrap(),
            json!("DEX")
        );
        assert_eq!(
            r(
                json!([{"op":"keys","table":"ab","unknown":"keep"}]),
                json!({"STR":1,"DEX":2})
            )
            .unwrap(),
            json!({"strength":1,"DEX":2})
        );
        assert_eq!(
            r(json!([{"op":"scale","factor":0.3048,"round":1}]), json!(30)).unwrap(),
            json!(9.1)
        );
        assert_eq!(
            r(json!([{"op":"scale","factor":0.3048,"round":0}]), json!(30)).unwrap(),
            json!(9)
        );
        assert_eq!(
            r(json!([{"op":"format","pattern":"{} ft."}]), json!(30)).unwrap(),
            json!("30 ft.")
        );
        assert_eq!(
            r(json!([{"op":"case","to":"kebab"}]), json!("Thieves' Cant")).unwrap(),
            json!("thieves-cant")
        );
        assert_eq!(
            r(json!([{"op":"case","to":"title"}]), json!("plate armor")).unwrap(),
            json!("Plate Armor")
        );
        assert_eq!(
            r(
                json!([{"op":"replace","find":" ","with":""}]),
                json!("1d8 + 3")
            )
            .unwrap(),
            json!("1d8+3")
        );
        assert_eq!(
            r(json!([{"op":"join","sep":", "}]), json!(["a", "b"])).unwrap(),
            json!("a, b")
        );
        assert_eq!(
            r(json!([{"op":"split","sep":", "}]), json!("a, b")).unwrap(),
            json!(["a", "b"])
        );
        assert_eq!(
            r(json!([{"op":"to_number"}]), json!("0.125")).unwrap(),
            json!(0.125)
        );
        assert_eq!(
            r(json!([{"op":"to_string"}]), json!(4)).unwrap(),
            json!("4")
        );
        assert_eq!(
            r(
                json!([{"op":"each","convert":[{"op":"case","to":"upper"}]}]),
                json!(["a"])
            )
            .unwrap(),
            json!(["A"])
        );
        assert_eq!(
            r(json!([{"op":"case","to":"upper"}]), Value::Null).unwrap(),
            Value::Null
        );
        assert!(r(json!([{"op":"case","to":"upper"}]), json!(3)).is_err());
    }
}
