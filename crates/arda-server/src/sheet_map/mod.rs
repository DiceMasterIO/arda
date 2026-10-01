//! The sheet mapping layer (goal 69; logic/16 §api-sheet-mapping): a
//! declarative file (`--sheet-mapping game.json`) that reshapes every Arda
//! `Npc` the server sends into the game's own creature schema: renames,
//! nesting, unit and enum conversions, constants and drops.
//!
//! The file is parsed and checked when the server starts, then run once
//! over probe NPCs (the `arda-npc` market town's notables and a spread of
//! its commoners); any problem refuses startup. Mapping is deterministic:
//! the same NPC and file always give the same bytes. See `API.md`
//! §"Sheet mapping" and the examples in `crates/arda-server/mappings/`.

mod check;
mod convert;
pub mod pointer;
pub mod spec;

use crate::error::{ServerError, ServerResult};
use axum::http::{HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use convert::Tables;
use serde::Serialize;
use serde_json::Value;
use spec::{Base, MappingFile, Rule};
use std::path::Path;

/// Response header naming the mapping a body went through.
pub const X_ARDA_SHEET_MAPPING: HeaderName = HeaderName::from_static("x-arda-sheet-mapping");

/// Largest mapping file read (1 MiB).
pub const MAX_FILE_BYTES: u64 = 1 << 20;

/// A checked mapping, ready to reshape NPCs.
#[derive(Debug, Clone)]
pub struct SheetMapping {
    file: MappingFile,
}

fn missing(at: &str, ptr: &str) -> String {
    format!("{at}: the source has nothing at {ptr:?} (mark the rule `optional`)")
}

fn template(t: &str, src: &Value, at: &str) -> Result<String, String> {
    let mut out = String::new();
    for (is_ptr, part) in check::template_parts(t)? {
        if !is_ptr {
            out.push_str(&part);
            continue;
        }
        match src.pointer(&part) {
            None => return Err(missing(at, &part)),
            Some(Value::Null) => {}
            Some(Value::String(s)) => out.push_str(s),
            Some(v @ (Value::Number(_) | Value::Bool(_))) => out.push_str(&v.to_string()),
            Some(v) => {
                return Err(format!(
                    "{at}: template placeholder {part:?} is a {}",
                    pointer::kind(v)
                ))
            }
        }
    }
    Ok(out)
}

fn matches(filter: Option<&std::collections::BTreeMap<String, Value>>, v: &Value) -> bool {
    filter.is_none_or(|f| f.iter().all(|(p, want)| v.pointer(p) == Some(want)))
}

fn fields(r: &Rule, sub: &[Rule], tables: &Tables, v: Value, at: &str) -> Result<Value, String> {
    let one = |el: &Value| -> Result<Value, String> {
        let mut out = Value::Object(serde_json::Map::new());
        rules(sub, tables, el, &mut out, &format!("{at}.fields"))?;
        Ok(out)
    };
    match v {
        Value::Null => Ok(Value::Null),
        Value::Array(items) => items
            .iter()
            .filter(|el| matches(r.filter.as_ref(), el))
            .map(one)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(_) if r.filter.is_some() => Err(format!(
            "{at}: `where` filters arrays, the source is an object"
        )),
        Value::Object(_) => one(&v),
        other => Err(format!(
            "{at}: `fields` needs an array or object, the source is a {}",
            pointer::kind(&other)
        )),
    }
}

fn rule(r: &Rule, tables: &Tables, src: &Value, out: &mut Value, at: &str) -> Result<(), String> {
    let value = if let Some(c) = &r.constant {
        c.clone()
    } else if let Some(t) = &r.template {
        Value::String(template(t, src, at)?)
    } else if let Some(f) = &r.from {
        match src.pointer(f) {
            None | Some(Value::Null) if r.optional => {
                if let Some(d) = &r.default {
                    pointer::write(out, &r.to, d.clone(), r.append)
                        .map_err(|e| format!("{at}: {e}"))?;
                }
                return Ok(());
            }
            None => return Err(missing(at, f)),
            Some(v) => v.clone(),
        }
    } else {
        return Err(format!("{at}: no source"));
    };
    let value = match &r.fields {
        Some(sub) => fields(r, sub, tables, value, at)?,
        None => value,
    };
    let value = convert::run(&r.convert, tables, value).map_err(|e| format!("{at}: {e}"))?;
    pointer::write(out, &r.to, value, r.append).map_err(|e| format!("{at}: {e}"))
}

fn rules(
    list: &[Rule],
    tables: &Tables,
    src: &Value,
    out: &mut Value,
    at: &str,
) -> Result<(), String> {
    for (i, r) in list.iter().enumerate() {
        rule(r, tables, src, out, &format!("{at}[{i}]"))?;
    }
    Ok(())
}

/// The NPCs every mapping must handle at startup: the `arda-npc` market
/// town's notables and every 37th inhabitant.
fn probes() -> Result<Vec<Value>, String> {
    let (seed, settlement, buildings) = arda_npc::sample::market_town();
    let g = arda_npc::Generator::new(seed, &settlement, &buildings).map_err(|e| e.to_string())?;
    let notables: Vec<_> = g.notable_ids().collect();
    let spread = g.ids().step_by(37);
    notables
        .into_iter()
        .chain(spread)
        .map(|id| {
            let npc = g.npc(id).map_err(|e| e.to_string())?;
            serde_json::to_value(npc).map_err(|e| e.to_string())
        })
        .collect()
}

impl SheetMapping {
    /// Parses, checks and dry-runs a mapping.
    ///
    /// # Errors
    /// [`ServerError::SheetMapping`] naming the first problem.
    pub fn from_json(text: &str) -> ServerResult<Self> {
        let mapping = Self::parse(text)?;
        for npc in probes().map_err(ServerError::Internal)? {
            mapping.apply(&npc).map_err(|e| {
                ServerError::SheetMapping(format!(
                    "{e} (probe NPC {})",
                    npc["id"].as_str().unwrap_or("?")
                ))
            })?;
        }
        Ok(mapping)
    }

    /// Parses and checks a mapping without the dry run.
    fn parse(text: &str) -> ServerResult<Self> {
        let file: MappingFile = serde_json::from_str(text)
            .map_err(|e| ServerError::SheetMapping(format!("not a mapping file: {e}")))?;
        check::file(&file).map_err(ServerError::SheetMapping)?;
        Ok(Self { file })
    }

    /// Reads and checks a mapping file.
    ///
    /// # Errors
    /// [`ServerError::SheetMapping`] for an unreadable, oversized or invalid file.
    pub fn load(path: &Path) -> ServerResult<Self> {
        let fail = |e: String| ServerError::SheetMapping(format!("{}: {e}", path.display()));
        let len = std::fs::metadata(path)
            .map_err(|e| fail(e.to_string()))?
            .len();
        if len > MAX_FILE_BYTES {
            return Err(fail(format!("{len} bytes, above {MAX_FILE_BYTES}")));
        }
        let text = std::fs::read_to_string(path).map_err(|e| fail(e.to_string()))?;
        Self::from_json(&text).map_err(|e| match e {
            ServerError::SheetMapping(m) => fail(m),
            other => other,
        })
    }

    /// The mapping's name (`X-Arda-Sheet-Mapping`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.file.name
    }

    /// Reshapes one Arda `Npc` (its JSON).
    ///
    /// # Errors
    /// The rule that could not be applied, and why.
    pub fn apply(&self, npc: &Value) -> Result<Value, String> {
        let mut out = match self.file.base {
            Base::Copy => npc.clone(),
            Base::Empty => Value::Object(serde_json::Map::new()),
        };
        rules(&self.file.rules, &self.file.tables, npc, &mut out, "rules")?;
        for d in &self.file.drop {
            pointer::remove(&mut out, d)?;
        }
        Ok(out)
    }

    /// Reshapes the NPCs of a body in place.
    ///
    /// # Errors
    /// [`ServerError::SheetMapping`] when a rule fails on a served NPC.
    pub fn reshape(&self, body: &mut Value, at: NpcAt) -> ServerResult<()> {
        let fail = |e: String| ServerError::SheetMapping(e);
        match at {
            NpcAt::Root => *body = self.apply(body).map_err(fail)?,
            NpcAt::List(list, field) => {
                let items = body
                    .pointer_mut(list)
                    .and_then(Value::as_array_mut)
                    .ok_or_else(|| ServerError::Internal(format!("no NPC list at {list}")))?;
                for item in items {
                    let slot = match field {
                        Some(f) => item
                            .get_mut(f)
                            .ok_or_else(|| ServerError::Internal(format!("no {f} in {list}")))?,
                        None => item,
                    };
                    *slot = self.apply(slot).map_err(fail)?;
                }
            }
        }
        Ok(())
    }
}

/// Where the `Npc` records sit in a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcAt {
    /// The body is one `Npc`.
    Root,
    /// An array at this pointer; each element is an `Npc`, or holds one in
    /// the named field.
    List(&'static str, Option<&'static str>),
}

/// The JSON response of a body holding NPCs: unchanged without a mapping,
/// reshaped (with `X-Arda-Sheet-Mapping`) with one.
///
/// # Errors
/// Serialisation or mapping failures.
pub fn respond<T: Serialize>(
    mapping: Option<&SheetMapping>,
    body: &T,
    at: NpcAt,
) -> ServerResult<Response> {
    let Some(m) = mapping else {
        return Ok(Json(body).into_response());
    };
    let mut v = serde_json::to_value(body).map_err(|e| ServerError::Internal(e.to_string()))?;
    m.reshape(&mut v, at)?;
    let mut r = Json(v).into_response();
    let name = HeaderValue::from_str(m.name()).map_err(|e| ServerError::Internal(e.to_string()))?;
    r.headers_mut().insert(X_ARDA_SHEET_MAPPING, name);
    Ok(r)
}

#[cfg(test)]
mod tests;
