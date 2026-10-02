# v0.6 integration: plains, worked land and field systems on one branch

- **What:** `integrate/v0.6` merges feat/v6-plains, then feat/v6-field-patterns (which contains feat/v6-midzoom-landuse), onto main's 0.5.0 (`c63a4b1`), with no conflicts. The default recipe stays 7; recipe 8 is opt-in.
- **Wedge-shaped fields (logic/17 §land-fields):** a cut other than a road may not meet its piece's boundary at under 38°, nor run within 12 squares of a boundary edge at such an angle. The check reads the clipped outline, the block outline and the true lines of the cuts above. The last-resort cuts obey it too, and a piece no cut can split stays one field. On MICRO 42, corners under 35° away from roads and rivers drop from 98 to 4 over six 4 km squares.
- **Verification:** `arda-fields/tests/partition.rs` `no_field_corner_is_a_wedge` (five synthetic scenarios, every corner 35° or more away from roads and rivers; it fails without the rule); the full gate and the server smoke test (integration-status §v0.6). Renders in `out/v6-int/`.
- **Skipped:** hedges and field walls drawn along the true diagonal instead of square-edge staircases. The reason is in integration-status §v0.6.
