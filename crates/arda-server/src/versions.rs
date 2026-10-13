//! Version families of the `/v1` surface and the build that serves them
//! (1.0 task 08; `docs/capstone/versioning.md`, logic/16 §api-versioning).
//!
//! [`VersionsDto`] lists every family's current version. `GET /v1/world`
//! carries it as `versions` so a client checks once at start, and the
//! TypeScript `index.ts` exports each entry as a constant. [`OPEN_ENUMS`]
//! names the enums that may gain values without a version bump.

use schemars::JsonSchema;
use serde::Serialize;
use ts_rs::TS;

/// Version of the `arda-server` build answering the request (Cargo package
/// version). `WorldInfo.arda_version` is the generator that wrote the world;
/// this is the binary that serves it and generates every tactical, NPC and
/// image body on demand.
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The current version of every family (`GET /v1/world` `versions`).
///
/// A family's version bumps when a body it covers removes or renames a
/// field, changes a field's type or meaning, or gains a value in a closed
/// enum; additions bump nothing (`docs/capstone/versioning.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct VersionsDto {
    /// Cell contract (`CellSample`, `PointSample`, area bodies, `Health`,
    /// `WorldInfo`).
    pub contract_version: u32,
    /// Tactical envelopes: `TacticalBlockDto`, `TacticalScene`,
    /// `DungeonDto`, `TacticalLibraryDto`, prefetch and layout listings.
    pub tactical_format: u32,
    /// `TacticalLayout` (`TacticalLayoutDto`).
    pub layout_schema: u32,
    /// `arda-scene` scene (`SceneDto.format_version`).
    pub scene_format: u32,
    /// `arda-scene` rules sidecar (`RulesSidecarDto.format_version`).
    pub rules_sidecar_format: u32,
    /// Bake job bodies (`BakeRequest`, `BakeAccepted`, `BakeStatus`).
    pub bake_format: u32,
    /// `Npc` and the bodies built from it (`Sheet`, `Population`,
    /// `GenerateNpcRequest`, `NpcDemo`).
    pub npc_format: u32,
    /// `NpcPage.format_version`.
    pub npc_page_format: u32,
    /// Settlement bodies (`SettlementList.format_version`).
    pub settlement_format: u32,
    /// Realm bodies (`RealmList.format_version`).
    pub realm_format: u32,
    /// `Gazetteer.format_version`.
    pub gazetteer_format: u32,
    /// `SchemaIndex.format_version`.
    pub schema_index_format: u32,
    /// Binary `ARDACOLS` layout of `?format=bin` area cells.
    pub columnar_layout_version: u32,
    /// Tactical library `catalog.json` `format_version` this server loads.
    pub library_catalogue_format: u32,
    /// `--sheet-mapping` file `version` this server reads.
    pub sheet_mapping_format: u32,
}

impl VersionsDto {
    /// The versions this build serves.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            contract_version: crate::contract::CONTRACT_VERSION,
            tactical_format: crate::tactical::dto::TACTICAL_FORMAT,
            layout_schema: crate::tactical::dto::LAYOUT_SCHEMA,
            scene_format: arda_scene::SCENE_FORMAT_VERSION,
            rules_sidecar_format: arda_scene::SIDECAR_FORMAT_VERSION,
            bake_format: crate::tactical::bake::BAKE_FORMAT,
            npc_format: crate::npc_dto::NPC_FORMAT,
            npc_page_format: crate::npcs::dto::NPC_PAGE_FORMAT,
            settlement_format: crate::schema::SETTLEMENT_FORMAT,
            realm_format: crate::schema::REALM_FORMAT,
            gazetteer_format: arda_settle::gazetteer::FORMAT_VERSION,
            schema_index_format: crate::schema::INDEX_FORMAT,
            columnar_layout_version: crate::columnar::LAYOUT_VERSION,
            library_catalogue_format: arda_tactical::catalog::FORMAT_VERSION,
            sheet_mapping_format: crate::sheet_map::spec::VERSION,
        }
    }

    /// Every family as `(key, version)`, in field order; the key is the
    /// JSON field name.
    #[must_use]
    pub const fn entries(&self) -> [(&'static str, u32); 15] {
        [
            ("contract_version", self.contract_version),
            ("tactical_format", self.tactical_format),
            ("layout_schema", self.layout_schema),
            ("scene_format", self.scene_format),
            ("rules_sidecar_format", self.rules_sidecar_format),
            ("bake_format", self.bake_format),
            ("npc_format", self.npc_format),
            ("npc_page_format", self.npc_page_format),
            ("settlement_format", self.settlement_format),
            ("realm_format", self.realm_format),
            ("gazetteer_format", self.gazetteer_format),
            ("schema_index_format", self.schema_index_format),
            ("columnar_layout_version", self.columnar_layout_version),
            ("library_catalogue_format", self.library_catalogue_format),
            ("sheet_mapping_format", self.sheet_mapping_format),
        ]
    }
}

/// Family key of the error bodies and of the route family itself; its
/// version is [`crate::dto::API_VERSION`], and bumping it means `/v2`.
pub const API_FAMILY: &str = "api_version";

/// Every family key with its current version as text: [`API_FAMILY`] plus
/// [`VersionsDto::entries`]. The schema freeze guard compares this with the
/// baseline's `versions.json`.
#[must_use]
pub fn family_versions() -> Vec<(&'static str, String)> {
    let mut out = vec![(API_FAMILY, crate::dto::API_VERSION.to_owned())];
    out.extend(
        VersionsDto::current()
            .entries()
            .iter()
            .map(|&(k, v)| (k, v.to_string())),
    );
    out
}

/// How firmly a route or schema is frozen (`docs/capstone/versioning.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    /// Frozen for 1.x under the versioning policy.
    Stable,
    /// Served, but outside the freeze: may change shape in a minor release.
    Unstable,
    /// Samples and test seams; may change or disappear in any release.
    Debug,
}

/// Open enums: `(schema $defs name, fallback value)`. These may gain values
/// in a minor release without a version bump; a client maps a value it does
/// not know to the fallback (`"-"` means: skip the entry). Every other enum
/// is closed: a new value bumps its family.
pub const OPEN_ENUMS: &[(&str, &str)] = &[
    ("AssetClassDto", "prop"),
    ("Biome", "temperate"),
    ("BlockerKindDto", "prop"),
    ("BuildingFunction", "house"),
    ("DistrictKind", "residential"),
    ("EntityRef", "-"),
    ("EventKind", "-"),
    ("FocalKind", "crossroads"),
    ("Function", "-"),
    ("GoodRole", "-"),
    ("JobCategory", "labour"),
    ("LandUseDto", "none"),
    ("Preset", "heartland"),
    ("RelationKind", "friend"),
    ("RoomPurposeDto", "hall"),
    ("SettlementFunction", "-"),
    ("Trend", "stable"),
];

/// Whether the enum named `name` (a schema `$defs` key) is open.
#[must_use]
pub fn is_open(name: &str) -> bool {
    OPEN_ENUMS.iter().any(|&(n, _)| n == name)
}
