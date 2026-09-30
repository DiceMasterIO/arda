## 2026-09-23 — Atlas land material readability

key: feat/atlas-land-materials@3c06909

- Added slope-derived exposed-rock colour, continuous rock-altitude blending, slope-weighted snow appearance and stronger bounded integer relief to Atlas land. Shared saved-world sampling, Classic, water ownership, formats and fixed rendering buffers remain unchanged.
- Verified 89 renderer tests, 56 facade/CLI tests, formatting and strict workspace Clippy. Two fresh independent review rounds found no actionable defects. A 32K export of the same default seed-42 world completed in 66.92 s; actual overview and matched regional renders were inspected.
- Refreshed architecture, models, data flow, tests, glossary, export behavior, map legend and current status. Coverage stamps reflect the committed renderer source; unchanged conventions/operations retain their existing contracts.
- The wider visual objective remains open: clearer materials do not resolve broad smooth mountains, parallel drainage, the large regional basin or missing ecological layers. Controlled terrain experiments remain isolated evidence, not production generator changes or completed visual acceptance.
