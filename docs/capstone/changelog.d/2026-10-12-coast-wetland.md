# Coast and wetland calibration

- **Request**: calibrate marsh, pools, beaches and the sea-level strip of tactical ground against measured world statistics (1.0 task 18, OI-41, OI-16).
- **Tool**: `arda-refine coastwet --world <dir> [--stride N]` prints deterministic JSON: signal shares by climate group, prior ground shares, pools per km2 of wet cells, and coast strip width per saved shore class. Baseline and retuned numbers for MICRO seed 42 and the same seed at latitudes 15-35 are in `logic/09-tactical-refinement.md` §biome-calibration.
- **Signals**: marsh wetness band 0.8-0.98 is now 0.5-0.7 (lowland marshy share 1.9 % to 4.5 %), and marsh cover obeys the 12 degree slope limit. In a world with `terrain/shore.bin` the coast signal reads the distance to the sea (4-cell reach) and the saved shore class: sand beaches are 3 cells deep over firm ground (89-92 % sand on beach cells, was 0 %), shingle and salt marsh 2 cells with their own ground, cliffs and rocky shores bare rock and no sand. Worlds without the layer keep the old height rule and weights.
- **Source**: `Source::shore` and `Source::has_shore` are new required methods; `GridSource::set_shore` builds fixtures.
- **Fixtures**: `tests/biome.rs` damp-meadow wetness 150 became 110 (150 is now partly marsh by design); the synthetic world's river row (wetness 200) now reads as marshy.
- **Not changed**: shore formation and cover marsh (recipe-gated). The sand beach share of the coast (5-6 % against about a third) is a recipe-9 candidate, recorded in OI-41. No golden changed.
