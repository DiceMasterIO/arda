//! `notables.json`, one settlement a line behind an index, read lazily
//! (review 3.3-10).
//!
//! At default world size the stored notables run to about half a gigabyte
//! of JSON, which the server used to parse whole at start. The file is
//! now written compact, one settlement per line, after a header line that
//! holds the format, the seed, the settlement stamp it was built from and
//! an index: per settlement its notable ids, its plan fingerprint and the
//! byte range of its line. A reader parses the header alone, finds a
//! notable by id through the index, and parses a settlement's line the
//! first time it is asked for. The whole file is still one valid JSON
//! document of the [`NotablesFile`] shape.
//!
//! Files of format 1 (pretty, no index) are read whole, as before.

use crate::files::StoredNotable;
use crate::PeopleError;
use arda_ids::SettlementId;
use arda_npc::{Npc, NpcId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{BufRead, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Version of `notables.json` this crate writes: 2, one settlement a line
/// behind an index.
pub const NOTABLES_FORMAT: u32 = 2;

/// One settlement's entry in the header index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotableIndex {
    /// The settlement.
    pub settlement_id: SettlementId,
    /// Fingerprint of the plan buildings its notables were generated in
    /// (`town::fingerprint`); `None` when they came from the building mix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_fingerprint: Option<String>,
    /// Its notables' ids, in order.
    pub npcs: Vec<NpcId>,
    /// Offset of its line after the header line, bytes.
    pub at: u64,
    /// Length of its line, bytes (without the separator).
    pub len: u64,
}

/// `notables.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotablesFile {
    /// Format version.
    pub format_version: u32,
    /// World seed, a decimal string.
    pub seed: String,
    /// Stamp id of the settlement files the notables were built from
    /// (`arda_settle::stamp::Stamp::id`); absent in older files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settle_stamp: Option<String>,
    /// Per settlement, in settlement order: where its line is. Empty in
    /// format 1.
    #[serde(default)]
    pub index: Vec<NotableIndex>,
    /// Per settlement, in settlement order.
    pub settlements: Vec<StoredNotable>,
}

/// The bytes of `file` as format 2: a header line with the index, then one
/// compact settlement a line. The result parses as a [`NotablesFile`].
///
/// # Errors
/// Serialisation errors (named after `path`).
pub fn encode(path: &Path, file: &NotablesFile) -> Result<Vec<u8>, PeopleError> {
    let fail = |e: serde_json::Error| PeopleError::format(path, e);
    let lines: Vec<Vec<u8>> = file
        .settlements
        .iter()
        .map(serde_json::to_vec)
        .collect::<Result<_, _>>()
        .map_err(fail)?;
    let mut index = Vec::with_capacity(lines.len());
    let mut at = 0_u64;
    for (s, line) in file.settlements.iter().zip(&lines) {
        let len = u64::try_from(line.len()).unwrap_or(u64::MAX);
        index.push(NotableIndex {
            settlement_id: s.settlement_id,
            plan_fingerprint: s.plan_fingerprint.clone(),
            npcs: s.npcs.iter().map(|n| n.id).collect(),
            at,
            len,
        });
        at += len + 2;
    }
    let header = NotablesFile {
        format_version: NOTABLES_FORMAT,
        seed: file.seed.clone(),
        settle_stamp: file.settle_stamp.clone(),
        index,
        settlements: Vec::new(),
    };
    let mut out = serde_json::to_vec(&header).map_err(fail)?;
    // `…,"settlements":[]}` loses its `]}`: the lines follow.
    out.truncate(out.len().saturating_sub(2));
    out.push(b'\n');
    let n = lines.len();
    for (k, line) in lines.into_iter().enumerate() {
        out.extend_from_slice(&line);
        out.extend_from_slice(if k + 1 < n { b",\n" } else { b"\n" });
    }
    out.extend_from_slice(b"]}\n");
    Ok(out)
}

/// A `notables.json` opened for lazy reading.
#[derive(Debug)]
pub struct Notables {
    path: PathBuf,
    /// Format version of the file.
    pub format_version: u32,
    /// World seed, a decimal string.
    pub seed: String,
    /// Stamp id of the settlement files it was built from.
    pub settle_stamp: Option<String>,
    index: Vec<NotableIndex>,
    body: u64,
    by_settlement: BTreeMap<u64, usize>,
    by_npc: BTreeMap<NpcId, usize>,
    lines: Vec<OnceLock<StoredNotable>>,
}

impl Notables {
    /// Opens `path`: parses the header line of a format-2 file, or the
    /// whole of a format-1 file.
    ///
    /// # Errors
    /// I/O or format errors.
    pub fn open(path: &Path) -> Result<Self, PeopleError> {
        let file = std::fs::File::open(path).map_err(|e| PeopleError::io(path, e))?;
        let mut r = std::io::BufReader::new(file);
        let mut head = String::new();
        r.read_line(&mut head)
            .map_err(|e| PeopleError::io(path, e))?;
        let head_len = head.len();
        let indexed = head.trim_end().ends_with("\"settlements\":[");
        let (header, preloaded) = if indexed {
            let text = format!("{}]}}", head.trim_end());
            let h: NotablesFile =
                serde_json::from_str(&text).map_err(|e| PeopleError::format(path, e))?;
            (h, None)
        } else {
            let mut rest = head.into_bytes();
            r.read_to_end(&mut rest)
                .map_err(|e| PeopleError::io(path, e))?;
            let mut whole: NotablesFile =
                serde_json::from_slice(&rest).map_err(|e| PeopleError::format(path, e))?;
            whole.index = whole
                .settlements
                .iter()
                .map(|s| NotableIndex {
                    settlement_id: s.settlement_id,
                    plan_fingerprint: s.plan_fingerprint.clone(),
                    npcs: s.npcs.iter().map(|n| n.id).collect(),
                    at: 0,
                    len: 0,
                })
                .collect();
            let all = std::mem::take(&mut whole.settlements);
            (whole, Some(all))
        };
        let body = if indexed {
            u64::try_from(head_len).unwrap_or(u64::MAX)
        } else {
            0
        };
        let mut by_settlement = BTreeMap::new();
        let mut by_npc = BTreeMap::new();
        for (k, e) in header.index.iter().enumerate() {
            by_settlement.insert(e.settlement_id.get(), k);
            for &id in &e.npcs {
                by_npc.insert(id, k);
            }
        }
        let lines: Vec<OnceLock<StoredNotable>> = match preloaded {
            Some(all) => all.into_iter().map(OnceLock::from).collect(),
            None => header.index.iter().map(|_| OnceLock::new()).collect(),
        };
        Ok(Self {
            path: path.to_path_buf(),
            format_version: header.format_version,
            seed: header.seed,
            settle_stamp: header.settle_stamp,
            index: header.index,
            body,
            by_settlement,
            by_npc,
            lines,
        })
    }

    /// Settlements listed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether no settlement is listed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Settlements whose line has been parsed so far.
    #[must_use]
    pub fn loaded(&self) -> usize {
        self.lines.iter().filter(|l| l.get().is_some()).count()
    }

    /// The index entry of settlement `id`, without reading its line.
    #[must_use]
    pub fn entry(&self, id: u64) -> Option<&NotableIndex> {
        self.by_settlement.get(&id).and_then(|&k| self.index.get(k))
    }

    /// The stored notables of settlement `id`, parsed on first use.
    ///
    /// # Errors
    /// I/O or format errors reading its line; a line of another settlement.
    pub fn of(&self, id: u64) -> Result<Option<&StoredNotable>, PeopleError> {
        match self.by_settlement.get(&id) {
            Some(&k) => self.line(k).map(Some),
            None => Ok(None),
        }
    }

    /// A stored notable by id and its settlement.
    ///
    /// # Errors
    /// As [`Notables::of`].
    pub fn notable(&self, id: NpcId) -> Result<Option<(&Npc, SettlementId)>, PeopleError> {
        let Some(&k) = self.by_npc.get(&id) else {
            return Ok(None);
        };
        let s = self.line(k)?;
        Ok(s.npcs
            .iter()
            .find(|n| n.id == id)
            .map(|n| (n, s.settlement_id)))
    }

    fn line(&self, k: usize) -> Result<&StoredNotable, PeopleError> {
        let cell = &self.lines[k];
        if let Some(s) = cell.get() {
            return Ok(s);
        }
        let e = &self.index[k];
        let mut f =
            std::fs::File::open(&self.path).map_err(|err| PeopleError::io(&self.path, err))?;
        f.seek(SeekFrom::Start(self.body + e.at))
            .map_err(|err| PeopleError::io(&self.path, err))?;
        let mut buf = vec![0_u8; usize::try_from(e.len).unwrap_or(0)];
        f.read_exact(&mut buf)
            .map_err(|err| PeopleError::io(&self.path, err))?;
        let s: StoredNotable =
            serde_json::from_slice(&buf).map_err(|err| PeopleError::format(&self.path, err))?;
        if s.settlement_id != e.settlement_id {
            return Err(PeopleError::format(
                &self.path,
                format!(
                    "line of settlement {} holds settlement {}",
                    e.settlement_id, s.settlement_id
                ),
            ));
        }
        let _ = cell.set(s);
        cell.get()
            .ok_or_else(|| PeopleError::format(&self.path, "a notables line vanished"))
    }
}
