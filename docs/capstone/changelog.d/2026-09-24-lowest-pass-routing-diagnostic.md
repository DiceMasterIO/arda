# Lowest-pass terrain-routing diagnosis and controlled comparisons

- **What:** isolated source experiments identify and correct overly high basin reconnections; reference-quality terrain remains unfinished.
- **Diagnosis:** old routes exceed true minimum pass for 24,466 central strict-source sites (977.026 km²), median positive excess 317.622 m and maximum 1908.449 m. The only raw-versus-model mesh difference connects fixed hull outlets and changes no spill level.
- **Approach:** deterministic basin-pass spanning tree with original receiver-path reversal, immutable bed during routing, unchanged strict downhill ties and erosion law. This is a declared integrated-drainage landscape-evolution approximation, not annual physical flux.
- **Verification:** six small routing fixtures, nine runner tests, repeated smoke and independent all 148,992 node minimax/local-edge/root/acyclic checks pass. Both 100-step paired runs reproduce retained old source fields and Atlas PNGs exactly; root area/material/height ledgers pass.
- **Strict source:** native central geometric fill 6.1516% → 0.3184%, minima 263 → 14; raster fill 7.5813% → 2.7061%. Paired runtime 15.984 s.
- **Dense source:** unchanged previous source rechecked because the same routing defect affected it; native central fill 9.6284% → 0.3810%, minima 371 → 27; raster fill 11.0409% → 2.7287%. Paired runtime 15.840 s.
- **Visual outcome:** valleys connect more coherently, but repeated cliff/bench bands and narrow incisions remain. Neither source is adopted or meets the reference. Material-contact attribution and drainage-preserving raster transfer remain open.
- **Scope:** production remains at `14a70b9`; no saved-world changes, source coefficient sweeps, full-world regeneration or new canonical-water solve. Source, binaries, raw fields, actual Atlas images, scripts, reviews and hashes retained under `features/2026-09-23-terrain-corrections/evidence/{native-basin-route-audit,lowest-pass-source,lowest-pass-dense-source}`.
- **Reference refreshed:** `open-items.md`; active feature plan records completed checks and visual limits.
