## 2026-09-30 - fix: world physics that only showed at full size
key: integrate/2026-09-30-world-full-size

Found by the seed-42 full-size run (500 × 1000 km) and reproduced on a 250 × 500 km world, which runs in 10 minutes. Not signed off: the look awaits the maintainer.

- **Deep closed basins** (logic/02 §fine-formation basins). Only rim-connected water below the −120 m lowstand is fixed ocean. An enclosed macro depression deeper than that used to be fixed as sea floor and then lifted 150–200 m by the final fills. That produced the 60 km "delta plain" at 63 m, dead flat, with straight channels and chains of tiny oxbows. It now forms as land, and on seed 42 it floods as a long sound.
- **Fill flats** (§fine-formation flats). Flats are regraded as cost-weighted geodesics, at 1–5 mm per step from rotated noise, so rivers wander and converge across them instead of following grid geodesics.
- **Deltas** (§world-water deltas).
  - The lobe's reach varies with bearing, giving about five sub-lobes, so the front has no noise holes.
  - The plain is concave.
  - A tree of distributaries bifurcates up to three times. The channels bend 28–36° at eleven widths and wander at fan scale.
  - Levees stand 1.4–0.4 m above interdistributary basins.
  - The tidal reach is cut to −1.5 m, so the distal plain splits into delta islands by open water. Sheltered delta shores are marsh.
- **Oxbows** (§world-water oxbows). Each crescent is protected whole and carved with a flat bottom.
  - A loop survives only if it is one 8-connected 100 m lake of at least eight samples that holds at least 0.3 m of water before it spills.
  - At most one loop per two wavelengths.
- **Beaches** (§shore classes). A low headland keeps a beach only on a built sand body or within 1.5 km of a mouth of 20 km². The survey's sea is 8-connected, as in the hydrology.
- **Sampled drainage** (§sampled drainage). The sea is 8-connected, so a lagoon joined across a diagonal inlet is no longer filled into a 100 m grid texture (seed-5 MICRO). Pockets are levelled whole, and the fine fill and sampled pass always run twice.
- **Terrace risers** (§terraces). Risers are read at a noise-shifted height, so they scallop instead of drawing straight lines along planar valley sides (seed-3 MICRO).
- **Channel initiation** (§hillslopes).
  - Maturity scales the initiation threshold 1–2× (it was 1–4×).
  - Low relief raises it up to 24×, cubically.
  - Strong relief lowers it to 0.35×.
  - Slopes over 22° divide it by (S/0.4)².
- **Arc volcanoes** (§margins). A straight subaerial cone reaches the shore at 0.35 R. The concave cone from a 1–4 km floor stood above sea only within 0.4 km of its summit, so every full-size arc ended under water, even after the littoral fix.
- **Memory.** The finest level no longer allocates a zero `u64` area vector (2.6 GB at full size) for roughening.
- **world_metrics** reads lake origins from `water.bin`: 540 → 0 unexplained lakes at full size. It reports barrier and delta islands apart from bays and headlands, and gives class breakdowns.
