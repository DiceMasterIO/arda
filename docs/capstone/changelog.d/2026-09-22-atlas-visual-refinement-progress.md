## 2026-09-22 - progress: Atlas visual fidelity
key: progress/2026-09-22-atlas-visual-fidelity@9a16883

- User objective remains active: finish the whole rendering feature until it meets the selected atlas reference; the completed first pass does not establish final visual acceptance.
- `crates/arda-render/src/atlas.rs`: clearer elevation colours and a piecewise saved-depth sea ramp replace the compressed linear sea palette; lighting, masks, Classic and APIs remain unchanged.
- `crates/arda-render/src/atlas/tests.rs`: verifies depth interpolation/joins/extremes and updates literal RGB records; preserves the adjacent-row corner regression oracle with a locally shifted planar fixture.
- Source increment: `9a16883`, 80 renderer tests passed, formatting and strict renderer Clippy passed; an independent Sol reviewer found no actionable issue.
- Real evidence: four isolated actual-CLI variants separate palette from lighting; production area/overview match the palette-only prototype, Classic matches its pre-Atlas hash, and all 55 input files remain unchanged.
- Rejected for production: stronger display exaggeration/lighting, because it accentuates the saved straight gullies without supplying reference-quality terrain.
- Diagnosis: the visible outer ocean strip matches the generator's two-coarse-cell forced ocean rim at approximately -1,050 m; the cause of straight terrestrial incision remains unproven.
- A fixed default seed-42 500×1000 km world generation was started to broaden evidence; no seed reroll, terrain code change or completed-world claim.
- `features/2026-09-22-atlas-visual-fidelity/goal-audit.md`: records requirement-by-requirement incompleteness, evidence and next actions; local probes, commands, sources, hashes and receipts are retained.
- Broader goal remains incomplete: coast/terrain fidelity, distracting boundary patterns, representative-world acceptance and user visual acceptance still require work. No implement-completion marker is written.
