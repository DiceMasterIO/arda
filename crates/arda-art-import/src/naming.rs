//! The raw-file naming convention: what asset a file becomes.
//!
//! A file stem is `<asset spec>[__<anything>]`, or the spec followed by
//! further dot-separated parts that are ignored:
//!
//! | stem | becomes |
//! |---|---|
//! | `prop.anvil__comfy_00012_` | prop `prop.anvil` |
//! | `veg.tree_oak.v3` | vegetation `veg.tree_oak` |
//! | `ground.grass.anything` | a `grass` texture variant |
//! | `water.water_shallow.2` | a `water_shallow` texture variant |
//! | `wall.stone.corner__take2` | the `corner` piece of kit `stone` |
//!
//! A manifest entry can map any file name to an id instead.

use arda_tactical::catalog::WallRole;

/// What a raw file is imported as.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// A ground (or, for `water_*` keys, water) texture variant.
    Texture {
        /// The ground key it paints.
        key: String,
    },
    /// A wall-kit piece.
    Wall {
        /// Kit name.
        kit: String,
        /// Role in the kit.
        role: WallRole,
    },
    /// A prop cut-out, `prop.<name>`.
    Prop {
        /// The name after `prop.`.
        name: String,
    },
    /// A vegetation cut-out, `veg.<name>`.
    Vegetation {
        /// The name after `veg.`.
        name: String,
    },
}

impl Target {
    /// The base id: `prop.anvil`, `wall.stone.corner`, or `ground.grass` /
    /// `water.water_deep` for textures (variants append `.<n>`).
    #[must_use]
    pub fn base_id(&self) -> String {
        match self {
            Self::Texture { key } if is_water(key) => format!("water.{key}"),
            Self::Texture { key } => format!("ground.{key}"),
            Self::Wall { kit, role } => format!("wall.{kit}.{}", role_name(*role)),
            Self::Prop { name } => format!("prop.{name}"),
            Self::Vegetation { name } => format!("veg.{name}"),
        }
    }

    /// Whether this is a ground or water texture.
    #[must_use]
    pub fn is_texture(&self) -> bool {
        matches!(self, Self::Texture { .. })
    }
}

/// Water keys are the ones named `water_*`.
#[must_use]
pub fn is_water(key: &str) -> bool {
    key.starts_with("water_")
}

/// Parses an asset spec such as `wall.stone.corner` or a whole file stem.
///
/// # Errors
/// A message saying why the stem does not follow the convention.
pub fn parse(stem: &str) -> Result<Target, String> {
    let spec = stem.split("__").next().unwrap_or(stem);
    let parts: Vec<&str> = spec.split('.').collect();
    let word = |i: usize| parts.get(i).copied().filter(|w| valid_word(w));
    let need = |i: usize, what: &str| {
        word(i).ok_or_else(|| format!("`{stem}`: expected {what} after `{}.`", parts[0]))
    };
    match parts.first().copied() {
        Some("ground" | "water") => Ok(Target::Texture {
            key: need(1, "a ground key")?.to_string(),
        }),
        Some("prop") => Ok(Target::Prop {
            name: need(1, "a prop name")?.to_string(),
        }),
        Some("veg") => Ok(Target::Vegetation {
            name: need(1, "a vegetation name")?.to_string(),
        }),
        Some("wall") => {
            let kit = need(1, "a kit name")?.to_string();
            let role = parts
                .get(2)
                .and_then(|r| role_from(r))
                .ok_or_else(|| {
                    format!(
                        "`{stem}`: expected a role (run, door, window, gate, corner, tee, cross, end, post) after `wall.{kit}.`"
                    )
                })?;
            Ok(Target::Wall { kit, role })
        }
        _ => Err(format!(
            "`{stem}`: name files `ground.<key>`, `water.<key>`, `wall.<kit>.<role>`, `prop.<name>` or `veg.<name>`, or map them in the manifest"
        )),
    }
}

fn valid_word(w: &str) -> bool {
    !w.is_empty()
        && w.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The serde name of a wall role.
#[must_use]
pub fn role_name(role: WallRole) -> &'static str {
    match role {
        WallRole::Run => "run",
        WallRole::Door => "door",
        WallRole::Window => "window",
        WallRole::Gate => "gate",
        WallRole::Corner => "corner",
        WallRole::Tee => "tee",
        WallRole::Cross => "cross",
        WallRole::End => "end",
        WallRole::Post => "post",
    }
}

fn role_from(s: &str) -> Option<WallRole> {
    [
        WallRole::Run,
        WallRole::Door,
        WallRole::Window,
        WallRole::Gate,
        WallRole::Corner,
        WallRole::Tee,
        WallRole::Cross,
        WallRole::End,
        WallRole::Post,
    ]
    .into_iter()
    .find(|r| role_name(*r) == s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convention_examples_parse() {
        assert_eq!(
            parse("prop.anvil__comfy_00012_").unwrap(),
            Target::Prop {
                name: "anvil".into()
            }
        );
        assert_eq!(parse("veg.tree_oak.v3").unwrap().base_id(), "veg.tree_oak");
        assert_eq!(
            parse("ground.grass.whatever").unwrap().base_id(),
            "ground.grass"
        );
        assert_eq!(
            parse("ground.water_deep").unwrap().base_id(),
            "water.water_deep"
        );
        assert_eq!(
            parse("wall.stone.corner__take2").unwrap(),
            Target::Wall {
                kit: "stone".into(),
                role: WallRole::Corner
            }
        );
    }

    #[test]
    fn bad_names_explain_themselves() {
        assert!(parse("wall.stone.cornr").unwrap_err().contains("role"));
        assert!(parse("ComfyUI_00001_").unwrap_err().contains("manifest"));
        assert!(parse("prop.").is_err());
        assert!(parse("prop.Anvil").is_err());
    }
}
