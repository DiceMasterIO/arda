## 2026-09-23 — Natural Atlas river presentation

key: map/natural-rivers@92689f8a8142

- Source `92689f8` replaces hard Mid/Dark overview strokes with bounded saved-width variation, fractional bank coverage and a muted edge-to-core water-colour profile. Source routes, physical widths and water ownership remain unchanged; display shading is not measured channel depth. Maximum 14-row halos keep 256-row streaming bounded.
- Reject the width/palette-only intermediate candidate after native-pixel inspection: it still resembles an opaque strip. Accept the final water-profile treatment as a partial improvement in actual small, fitted 32K and native-pixel comparisons. Mountains and reference-quality acceptance remain open.
- Correct the zero-width synthetic discharge fallback to saturate at maximum display size when physical width exceeds u32; preserve Classic bytes.
- Verify103 renderer and 12 public export tests, strict renderer Clippy,formatting and diff checking. Two fresh final independent Sol reviews are dry.
- Verify two identical 15,522×32,768 outputs (326,768,717bytes; 80.75/80.68s), unchanged 523 saved files and byte-identicalClassic 2K. Exact commands,source hashes and render hashes remain in local evidence/natural-river-pass.
- Refresh rendering architecture, models, flow, operations, testing, export behavior, current status and index. Preserve prior implemented work; no whole-feature completion marker, push or PR.
