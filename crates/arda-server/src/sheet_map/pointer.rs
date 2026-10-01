//! JSON Pointer (RFC 6901) parsing, writing and removal for the mapping
//! layer. Reads use `serde_json::Value::pointer`.

use serde_json::{Map, Value};

/// The reference tokens of `ptr`, unescaped (`~1` is `/`, `~0` is `~`).
///
/// # Errors
/// A pointer that is neither `""` nor starts with `/`, or a bad escape.
pub fn tokens(ptr: &str) -> Result<Vec<String>, String> {
    if ptr.is_empty() {
        return Ok(Vec::new());
    }
    let Some(rest) = ptr.strip_prefix('/') else {
        return Err(format!("pointer {ptr:?} must be empty or start with '/'"));
    };
    rest.split('/')
        .map(|t| {
            let mut out = String::with_capacity(t.len());
            let mut chars = t.chars();
            while let Some(c) = chars.next() {
                if c == '~' {
                    match chars.next() {
                        Some('0') => out.push('~'),
                        Some('1') => out.push('/'),
                        _ => return Err(format!("pointer {ptr:?} has a bad '~' escape")),
                    }
                } else {
                    out.push(c);
                }
            }
            Ok(out)
        })
        .collect()
}

/// An output pointer: non-empty and well formed.
///
/// # Errors
/// As [`tokens`], or the empty pointer.
pub fn target(ptr: &str) -> Result<Vec<String>, String> {
    let t = tokens(ptr)?;
    if t.is_empty() {
        return Err("`to` must name a location below the root, not \"\"".into());
    }
    Ok(t)
}

fn index(token: &str, len: usize, ptr: &str) -> Result<usize, String> {
    if token == "-" {
        return Ok(len);
    }
    let canonical = token == "0" || (!token.starts_with('0') && !token.is_empty());
    match token.parse::<usize>() {
        Ok(i) if canonical && i <= len => Ok(i),
        Ok(i) if canonical => Err(format!(
            "{ptr}: index {i} is past the end of an array of {len}"
        )),
        _ => Err(format!("{ptr}: {token:?} is not an array index")),
    }
}

fn container_for(next: &str) -> Value {
    if next == "-" || next.bytes().all(|b| b.is_ascii_digit()) && !next.is_empty() {
        Value::Array(Vec::new())
    } else {
        Value::Object(Map::new())
    }
}

/// Writes `value` at `ptr` in `out`, creating missing objects and arrays
/// (an array when the next token is an index or `-`; `null` on the way is
/// replaced). With `append`, an array value extends an array already there.
///
/// # Errors
/// A malformed pointer, an index past the end, or a scalar on the path.
pub fn write(out: &mut Value, ptr: &str, value: Value, append: bool) -> Result<(), String> {
    let toks = target(ptr)?;
    let mut cur = out;
    for (i, tok) in toks.iter().enumerate() {
        let last = i + 1 == toks.len();
        if cur.is_null() {
            *cur = container_for(tok);
        }
        let fresh = || toks.get(i + 1).map_or(Value::Null, |n| container_for(n));
        cur = match cur {
            Value::Object(map) => {
                if last {
                    match (map.get_mut(tok.as_str()), value) {
                        (Some(Value::Array(have)), Value::Array(more)) if append => {
                            have.extend(more);
                        }
                        (_, v) => {
                            map.insert(tok.clone(), v);
                        }
                    }
                    return Ok(());
                }
                map.entry(tok.clone()).or_insert_with(fresh)
            }
            Value::Array(items) => {
                let at = index(tok, items.len(), ptr)?;
                if last {
                    match (items.get_mut(at), value) {
                        (Some(Value::Array(have)), Value::Array(more)) if append => {
                            have.extend(more);
                        }
                        (Some(slot), v) => *slot = v,
                        (None, v) => items.push(v),
                    }
                    return Ok(());
                }
                if at == items.len() {
                    items.push(fresh());
                }
                items
                    .get_mut(at)
                    .ok_or_else(|| format!("{ptr}: index {at} vanished"))?
            }
            other => return Err(format!("{ptr}: cannot write below a {} value", kind(other))),
        };
    }
    Ok(())
}

/// Removes the value at `ptr`; a missing location is left alone.
///
/// # Errors
/// A malformed pointer.
pub fn remove(out: &mut Value, ptr: &str) -> Result<(), String> {
    let toks = target(ptr)?;
    let Some((last, path)) = toks.split_last() else {
        return Ok(());
    };
    let mut cur = out;
    for tok in path {
        let next = match cur {
            Value::Object(map) => map.get_mut(tok.as_str()),
            Value::Array(items) => tok.parse::<usize>().ok().and_then(|i| items.get_mut(i)),
            _ => None,
        };
        match next {
            Some(v) => cur = v,
            None => return Ok(()),
        }
    }
    match cur {
        Value::Object(map) => {
            map.remove(last.as_str());
        }
        Value::Array(items) => {
            if let Some(i) = last.parse::<usize>().ok().filter(|&i| i < items.len()) {
                items.remove(i);
            }
        }
        _ => {}
    }
    Ok(())
}

/// A JSON type name for messages.
#[must_use]
pub fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn writes_create_objects_and_arrays_and_append_extends() {
        let mut v = json!({});
        write(&mut v, "/a/b", json!(1), false).unwrap();
        write(&mut v, "/ac/0/value", json!(12), false).unwrap();
        write(&mut v, "/ac/0/type", json!("armor"), false).unwrap();
        write(&mut v, "/list", json!([1]), false).unwrap();
        write(&mut v, "/list", json!([2, 3]), true).unwrap();
        write(&mut v, "/list/-", json!(4), false).unwrap();
        write(&mut v, "/x~1y", json!(true), false).unwrap();
        assert_eq!(
            v,
            json!({"a":{"b":1},"ac":[{"type":"armor","value":12}],"list":[1,2,3,4],"x/y":true})
        );
        assert!(write(&mut v, "/ac/5/value", json!(0), false).is_err());
        assert!(write(&mut v, "/a/b/c", json!(0), false).is_err());
        assert!(write(&mut v, "", json!(0), false).is_err());
        assert!(tokens("no-slash").is_err());
        assert!(tokens("/bad~2").is_err());
    }

    #[test]
    fn removal_ignores_missing_locations() {
        let mut v = json!({"a":{"b":1,"c":[1,2]}});
        remove(&mut v, "/a/c/0").unwrap();
        remove(&mut v, "/a/nope/x").unwrap();
        remove(&mut v, "/a/b").unwrap();
        assert_eq!(v, json!({"a":{"c":[2]}}));
    }
}
