## 2026-09-23 — Connected Atlas overview rivers

key: map/connected-atlas-rivers@14a70b9144dd

- Integrate saved channel strips and junction hulls into Atlas world-overview exports as `14a70b9144dd`. Complete 3×3 saved context, source clipping and exact rectangular output spans keep geometry connected across publication boundaries.
- Use 8× physical display widths capped 10,000 dm only in the new overview path. Preserve physical area coverage, Classic, direct legacy Atlas APIs and saved formats.
- Fix dense low-quality union work with a 64-piece subdivision trigger, maximum depth 6 and unchanged 250M work budget; omit no channels. Retain failed then passing 512 export evidence.
- Verify 108 renderer/12 public export tests, formatting, strict scoped Clippy, analytic rectangle and dense GEOS fixtures, plus two independent final dry reviews.
- Repeat full 32K output byte-for-byte in 93.19/92.62 s. Preserve all 523 saved files and selected Classic/physical-area controls.
- Refresh architecture, models, data-flow, testing, operations, export behavior, index and current status. Root accepts a partial river-geometry improvement; D8 angles, faint fitted tributaries and mountain morphology remain open.
