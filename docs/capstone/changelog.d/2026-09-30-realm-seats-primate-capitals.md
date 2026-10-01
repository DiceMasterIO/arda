## 2026-09-30 - feat: spread realm seats and primate capitals (goals 35, 38)
key: feat/2026-09-30-realm-seats

- New `seats.rs` and `market.rs` (logic/08 §realm-seats). The realm count follows people (24,000 a realm), habitable land (600 km² a realm) and towns (3 a realm). Seats are picked among the larger half of the towns by a size-weighted farthest-point pick over a terrain-aware market lattice. Each seat then moves within its own market area while that balances the areas' shares of people and land. A market area under 16,000 people has no core, so the land holds one realm fewer.
- New `primacy.rs`: within each realm the towns follow rank-size with the seat first. The seat grows into a primate city of at least 8,000 people, and the other towns rise by the fourth root of its growth. The extra people come from the realm's villages and hamlets. This runs before land use, so fields and roads serve the final populations. The city cap never demotes a seat.
- `stats.json` has a new `realm` block: cities per realm, seat spacing against the hexagonal lattice spacing, people-weighted distance to a seat, and realm land and people shares.
- New test `tests/realms_micro.rs`, run on seed 42 by default and on seeds 3 and 7 with `--ignored`. `tests/micro.rs` now checks that society history ends at the present partition and seats.
- MICRO results, before → after:
  - **Cities per realm:**
    - seed 42: [1, 0, 0] → [1, 1, 1];
    - seed 3: [1, 0, 0, 0] → [1, 1, 1, 1];
    - seed 7: [1, 1, 0, 0, 0] → [1, 1, 1, 1, 1].
  - **Nearest-seat distance, min / mean km:** seed 42 21.5 / 25.0 → 48.2 / 50.1; seed 3 42.8 / 45.9 → 42.8 / 45.9 (the same seats); seed 7 23.1 / 25.4 → 25.6 / 28.1.
  - **People-weighted p90 distance to a seat, km:** seed 42 54.4 → 32.4; seed 3 26.5 → 26.0; seed 7 57.5 → 42.2.
  - **Largest land share / largest-to-smallest ratio:** seed 42 0.75 / 5.98 → 0.45 / 2.26; seed 3 0.38 / 3.32 → 0.38 / 3.38; seed 7 0.42 / 7.94 → 0.36 / 3.79.
  - **Largest-to-smallest people ratio:** seed 42 2.35 → 2.06; seed 3 1.32 → 1.31; seed 7 3.18 → 1.90.
  - **Rank-size slope (R²):** seed 42 −1.00 (1.00) → −1.01 (0.85); seed 3 −1.00 → −0.98 (0.84); seed 7 −1.00 → −0.87 (0.83).
  - The urban share rises from 30 % to 37–42 %, because the city floor is fixed at 8,000 on worlds of 90,000–195,000 people.
