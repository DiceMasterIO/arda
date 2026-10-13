//! The schema freeze guard (1.0 task 08; `docs/capstone/versioning.md`
//! § Freeze guard): compares a baseline JSON Schema with the current one and
//! lists every change a `/v1` client could break on.
//!
//! Breaking: a removed property, a property newly in `required`, a changed
//! `type`, `format`, `const` or `$ref`, a removed enum value or union variant,
//! and a value or variant added to a closed enum ([`crate::versions::OPEN_ENUMS`]
//! lists the open ones). Additions of optional properties, schemas and
//! `$defs` pass. A removed `$defs` entry is reported where it was referenced.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// One side of a comparison: schema files by name, each schema's family,
/// and every family's version.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Schema documents by name.
    pub schemas: BTreeMap<String, Value>,
    /// Family key of each schema.
    pub families: BTreeMap<String, String>,
    /// Version of each family, as text.
    pub versions: BTreeMap<String, String>,
}

impl Snapshot {
    /// The family whose bump excuses a change inside `$defs/{def}`: that of
    /// the smallest schema (fewest `$defs`, then name) that is `def` itself
    /// or defines it, so a change to `MovementDto` is owned by `SceneDto`.
    fn home(&self, def: &str) -> Option<&String> {
        self.schemas
            .iter()
            .filter(|(n, v)| *n == def || v.get("$defs").and_then(|d| d.get(def)).is_some())
            .min_by_key(|(n, v)| {
                let defs = v
                    .get("$defs")
                    .and_then(Value::as_object)
                    .map_or(0, Map::len);
                (usize::from(n.as_str() != def), defs, (*n).clone())
            })
            .and_then(|(n, _)| self.families.get(n))
    }
}

/// Every breaking change from `old` to `new` that no family bump excuses,
/// as `"<schema>: <location>: <change> (family <f>)"`. A change is excused
/// when the schema's family or the family owning the changed `$defs` entry
/// ([`Snapshot::home`]) has a different version in `new` than in `old`.
pub fn unexcused(old: &Snapshot, new: &Snapshot, open: &dyn Fn(&str) -> bool) -> Vec<String> {
    let bumped = |f: Option<&String>| f.is_some_and(|f| old.versions.get(f) != new.versions.get(f));
    let mut out = Vec::new();
    for (name, before) in &old.schemas {
        let family = old.families.get(name);
        let Some(after) = new.schemas.get(name) else {
            if !bumped(family) {
                out.push(format!("{name}: schema removed (family {family:?})"));
            }
            continue;
        };
        for b in diff(name, before, after, open) {
            let home = b.def.as_deref().and_then(|d| old.home(d));
            if !bumped(family) && !bumped(home) {
                out.push(format!(
                    "{name}: {}: {} (family {family:?}{})",
                    if b.at.is_empty() { "/" } else { &b.at },
                    b.what,
                    home.map_or_else(String::new, |h| format!(", owner {h}"))
                ));
            }
        }
    }
    out
}

/// One breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Break {
    /// The `$defs` entry it lies in, or `None` for the schema's root.
    pub def: Option<String>,
    /// JSON-Pointer-like location inside the schema.
    pub at: String,
    /// What changed.
    pub what: String,
}

/// Every breaking change from `old` to `new` of the schema named `name`;
/// `open` says whether an enum (by `$defs` name) is open.
pub fn diff(name: &str, old: &Value, new: &Value, open: &dyn Fn(&str) -> bool) -> Vec<Break> {
    let mut out = Vec::new();
    let mut cx = Cx {
        def: None,
        enum_name: name.to_owned(),
        open,
        out: &mut out,
    };
    node(&mut cx, "", old, new);
    let empty = Map::new();
    let old_defs = old
        .get("$defs")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let new_defs = new
        .get("$defs")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    for (def, o) in old_defs {
        if let Some(n) = new_defs.get(def) {
            let mut cx = Cx {
                def: Some(def.clone()),
                enum_name: def.clone(),
                open,
                out: &mut out,
            };
            node(&mut cx, &format!("/$defs/{def}"), o, n);
        }
    }
    out
}

struct Cx<'a> {
    def: Option<String>,
    enum_name: String,
    open: &'a dyn Fn(&str) -> bool,
    out: &'a mut Vec<Break>,
}

impl Cx<'_> {
    fn push(&mut self, at: &str, what: String) {
        self.out.push(Break {
            def: self.def.clone(),
            at: at.to_owned(),
            what,
        });
    }
}

fn types(v: &Value) -> BTreeSet<String> {
    match v.get("type") {
        Some(Value::String(s)) => BTreeSet::from([s.clone()]),
        Some(Value::Array(a)) => a
            .iter()
            .map(|t| t.as_str().map_or_else(|| t.to_string(), str::to_owned))
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// The values of an enum: `enum`, or a `oneOf`/`anyOf` whose every branch
/// is a `const`.
fn enum_values(v: &Value) -> Option<BTreeSet<String>> {
    if let Some(Value::Array(a)) = v.get("enum") {
        return Some(a.iter().map(ToString::to_string).collect());
    }
    let branches = union(v)?;
    branches.iter().all(|b| b.get("const").is_some()).then(|| {
        branches
            .iter()
            .filter_map(|b| b.get("const"))
            .map(ToString::to_string)
            .collect()
    })
}

fn union(v: &Value) -> Option<&Vec<Value>> {
    v.get("oneOf")
        .or_else(|| v.get("anyOf"))
        .and_then(Value::as_array)
}

/// A stable key for a union branch: its `$ref`, `null`, its tag property's
/// `const`, or its index.
fn branch_key(i: usize, b: &Value) -> String {
    if let Some(r) = b.get("$ref").and_then(Value::as_str) {
        return r.to_owned();
    }
    if b.get("type").and_then(Value::as_str) == Some("null") {
        return "null".to_owned();
    }
    if let Some(props) = b.get("properties").and_then(Value::as_object) {
        for (p, s) in props {
            if let Some(c) = s.get("const") {
                return format!("{p}={c}");
            }
        }
        if props.len() == 1 {
            if let Some(p) = props.keys().next() {
                return format!("{{{p}}}");
            }
        }
    }
    format!("#{i}")
}

fn node(cx: &mut Cx<'_>, at: &str, old: &Value, new: &Value) {
    let (Some(o), Some(n)) = (old.as_object(), new.as_object()) else {
        if old != new {
            cx.push(at, format!("schema {old} became {new}"));
        }
        return;
    };
    for key in ["$ref", "format", "const"] {
        if o.get(key) != n.get(key) {
            cx.push(
                at,
                format!(
                    "{key} {} became {}",
                    o.get(key).unwrap_or(&Value::Null),
                    n.get(key).unwrap_or(&Value::Null)
                ),
            );
        }
    }
    if types(old) != types(new) {
        cx.push(at, format!("type {:?} became {:?}", types(old), types(new)));
    }
    if let Some(ov) = enum_values(old) {
        match enum_values(new) {
            None => cx.push(at, "enum is no longer an enum".to_owned()),
            Some(nv) => {
                for gone in ov.difference(&nv) {
                    cx.push(at, format!("enum value {gone} removed"));
                }
                let is_open = (cx.open)(&cx.enum_name);
                for added in nv.difference(&ov) {
                    if !is_open {
                        cx.push(
                            at,
                            format!("value {added} added to closed enum {}", cx.enum_name),
                        );
                    }
                }
            }
        }
    } else if let Some(ob) = union(old) {
        variants(cx, at, ob, union(new).map_or(&[][..], Vec::as_slice));
    }
    properties(cx, at, o, n);
    for key in ["items", "additionalProperties"] {
        match (o.get(key), n.get(key)) {
            (Some(a), Some(b)) => node(cx, &format!("{at}/{key}"), a, b),
            (Some(a), None) => cx.push(at, format!("{key} {a} removed")),
            _ => {}
        }
    }
    let prefix = |m: &Map<String, Value>| {
        m.get("prefixItems")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let (op, np) = (prefix(o), prefix(n));
    if op.len() != np.len() {
        cx.push(at, format!("tuple length {} became {}", op.len(), np.len()));
    }
    for (i, (a, b)) in op.iter().zip(&np).enumerate() {
        node(cx, &format!("{at}/prefixItems/{i}"), a, b);
    }
}

fn variants(cx: &mut Cx<'_>, at: &str, old: &[Value], new: &[Value]) {
    let keyed = |list: &[Value]| -> Vec<(String, Value)> {
        list.iter()
            .enumerate()
            .map(|(i, b)| (branch_key(i, b), b.clone()))
            .collect()
    };
    let (ok, nk) = (keyed(old), keyed(new));
    let is_open = (cx.open)(&cx.enum_name);
    for (k, a) in &ok {
        match nk.iter().find(|(key, _)| key == k) {
            Some((_, b)) => node(cx, &format!("{at}/oneOf[{k}]"), a, b),
            None => cx.push(at, format!("union variant {k} removed")),
        }
    }
    for (k, _) in &nk {
        if !is_open && !ok.iter().any(|(key, _)| key == k) {
            cx.push(
                at,
                format!("variant {k} added to closed union {}", cx.enum_name),
            );
        }
    }
}

fn properties(cx: &mut Cx<'_>, at: &str, o: &Map<String, Value>, n: &Map<String, Value>) {
    let empty = Map::new();
    let op = o
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let np = n
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    for (p, a) in op {
        match np.get(p) {
            Some(b) => node(cx, &format!("{at}/properties/{p}"), a, b),
            None => cx.push(at, format!("property {p} removed")),
        }
    }
    let required = |m: &Map<String, Value>| -> BTreeSet<String> {
        m.get("required")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    for p in required(n).difference(&required(o)) {
        cx.push(at, format!("property {p} is newly required"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base() -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "n": {"type": "integer", "format": "uint32"},
                "kind": {"$ref": "#/$defs/Kind"},
                "land": {"$ref": "#/$defs/LandUseDto"}
            },
            "required": ["id", "kind"],
            "$defs": {
                "Kind": {"oneOf": [{"const": "a", "type": "string"}, {"const": "b", "type": "string"}]},
                "LandUseDto": {"oneOf": [{"const": "none", "type": "string"}]}
            }
        })
    }

    fn breaks(new: &Value) -> Vec<Break> {
        diff("Body", &base(), new, &crate::versions::is_open)
    }

    #[test]
    fn an_unchanged_schema_and_an_added_optional_property_pass() {
        assert!(breaks(&base()).is_empty());
        let mut v = base();
        v["properties"]["extra"] = json!({"type": ["string", "null"]});
        v["$defs"]["New"] = json!({"type": "object"});
        assert!(breaks(&v).is_empty(), "{:?}", breaks(&v));
    }

    #[test]
    fn a_removed_property_fails() {
        let mut v = base();
        v["properties"].as_object_mut().unwrap().remove("n");
        let b = breaks(&v);
        assert_eq!(b.len(), 1, "{b:?}");
        assert!(b[0].what.contains("property n removed"));
    }

    #[test]
    fn a_newly_required_property_fails() {
        let mut v = base();
        v["required"] = json!(["id", "kind", "n"]);
        let b = breaks(&v);
        assert_eq!(b.len(), 1, "{b:?}");
        assert!(b[0].what.contains("n is newly required"));
    }

    #[test]
    fn a_type_or_format_change_fails() {
        let mut v = base();
        v["properties"]["id"] = json!({"type": "integer"});
        assert_eq!(breaks(&v).len(), 1);
        let mut v = base();
        v["properties"]["n"]["format"] = json!("uint64");
        assert_eq!(breaks(&v).len(), 1);
        let mut v = base();
        v["properties"]["id"] = json!({"type": ["string", "null"]});
        assert_eq!(breaks(&v).len(), 1, "becoming nullable is a type change");
    }

    #[test]
    fn a_removed_enum_value_fails() {
        let mut v = base();
        v["$defs"]["Kind"]["oneOf"].as_array_mut().unwrap().pop();
        let b = breaks(&v);
        assert_eq!(b.len(), 1, "{b:?}");
        assert_eq!(b[0].def.as_deref(), Some("Kind"));
        assert!(b[0].what.contains("removed"));
    }

    #[test]
    fn a_value_added_to_a_closed_enum_fails_and_to_an_open_one_passes() {
        let mut v = base();
        v["$defs"]["Kind"]["oneOf"]
            .as_array_mut()
            .unwrap()
            .push(json!({"const": "c", "type": "string"}));
        let b = breaks(&v);
        assert_eq!(b.len(), 1, "{b:?}");
        assert!(b[0].what.contains("closed enum Kind"));
        let mut v = base();
        v["$defs"]["LandUseDto"]["oneOf"]
            .as_array_mut()
            .unwrap()
            .push(json!({"const": "quarry", "type": "string"}));
        assert!(breaks(&v).is_empty(), "LandUseDto is open");
    }

    fn snapshot(schema: Value, version: &str) -> Snapshot {
        Snapshot {
            schemas: BTreeMap::from([("Body".to_owned(), schema)]),
            families: BTreeMap::from([("Body".to_owned(), "body_format".to_owned())]),
            versions: BTreeMap::from([("body_format".to_owned(), version.to_owned())]),
        }
    }

    #[test]
    fn a_family_bump_excuses_a_break_and_nothing_else_does() {
        let mut v = base();
        v["properties"].as_object_mut().unwrap().remove("n");
        let open = &crate::versions::is_open;
        let old = snapshot(base(), "1");
        assert_eq!(unexcused(&old, &snapshot(v.clone(), "1"), open).len(), 1);
        assert!(unexcused(&old, &snapshot(v, "2"), open).is_empty());
        let gone = Snapshot {
            schemas: BTreeMap::new(),
            ..snapshot(base(), "1")
        };
        assert_eq!(unexcused(&old, &gone, open).len(), 1, "a removed schema");
    }

    #[test]
    fn a_def_change_is_owned_by_the_smallest_schema_defining_it() {
        let scene = json!({"type": "object", "$defs": {"Move": {"oneOf": [{"const": "walk"}]}}});
        let block = json!({"type": "object", "$defs": {
            "Move": {"oneOf": [{"const": "walk"}]}, "SceneDto": {}, "Other": {}}});
        let old = Snapshot {
            schemas: BTreeMap::from([
                ("SceneDto".to_owned(), scene.clone()),
                ("Block".to_owned(), block.clone()),
            ]),
            families: BTreeMap::from([
                ("SceneDto".to_owned(), "scene_format".to_owned()),
                ("Block".to_owned(), "tactical_format".to_owned()),
            ]),
            versions: BTreeMap::from([
                ("scene_format".to_owned(), "1".to_owned()),
                ("tactical_format".to_owned(), "1".to_owned()),
            ]),
        };
        assert_eq!(old.home("Move").map(String::as_str), Some("scene_format"));
        assert_eq!(
            old.home("SceneDto").map(String::as_str),
            Some("scene_format")
        );
        let mut new = old.clone();
        for (_, v) in new.schemas.iter_mut() {
            v["$defs"]["Move"]["oneOf"] = json!([{"const": "walk"}, {"const": "fly"}]);
        }
        let open = |_: &str| false;
        assert_eq!(unexcused(&old, &new, &open).len(), 2);
        new.versions
            .insert("scene_format".to_owned(), "2".to_owned());
        assert!(unexcused(&old, &new, &open).is_empty());
    }

    #[test]
    fn a_retyped_reference_fails() {
        let mut v = base();
        v["properties"]["kind"] = json!({"$ref": "#/$defs/Other"});
        assert_eq!(breaks(&v).len(), 1);
    }
}
