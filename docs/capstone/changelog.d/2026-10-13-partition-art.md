## 2026-10-13 - Tier-5 partition art prepared
- **Request**: 1.0 task 06, OI-32. Interior walls inside one building are tagged `partition`; until `timber_partition`, `wattle_partition` and `adobe_partition` art exists the compositor squeezes the kit's own piece to half thickness.
- **Importer**: `WALL_KITS` gains `log`, `adobe` and the three partition kits, so `wall.<kit>_partition.<role>` files are no longer flagged "not in the vocabulary"; coverage slots grow by five. A partition kit with missing roles is reported, not rejected (test `partition_kits_are_in_the_vocabulary`).
- **Compositor**: test that a library holding thin `timber_partition` pieces draws them as painted (thinner than the exterior piece) and that without them the half-thickness fallback stays.
- **art-gen**: `tiers.about` and the `expansion.mjs` header now describe tier 5 (57 images); plan.md has the tier-5 dry-run command and the thin-band redo rule. The images are not generated yet; the fallback stays until they are.
- **Closed**: OI-32 (code and prompt side; generation and import follow with the maintainer's approval).
