# 2026-10-13 — Housekeeping before 1.0

- **tests**: `tests/zoom_land.rs` gains a module-header note: it is the `zoom_land` module of the `zoom_continuity` target (`mod zoom_land;`, `tests/zoom_continuity.rs`), not a target of its own, and is run with `cargo test --release --test zoom_continuity -- --ignored`. It stays out of the root `[[test]]` list; moving it was ruled out because 06-testing, logic/17 and open-items cite the current path.
- **docs**: `09-interfaces.md` gives `GET /v1/tactical/window/tiles/{z}/{x}/{y}.webp` the path and query table it lacked (same fields as `/window.png`).
- **docs**: the 2026-09-25 terrain-study fragment is folded into `changelog.md` as a dated entry, trimmed (no `codex/` branch name, no agent or model names; commit `52eab74` kept), and the fragment removed.
- **capstone schema**: `logic/README.md` is checked as a scenario by `~/.agents/skills/core/references/schema.txt` and reports eight missing headings. The upstream fix is a `logic-readme` record above `logic-scenario` (`match logic/README.md`, `keys! generated_date`); the installed skill is left unedited and the patch goes to `GentBajko/capstone`. In a scratch copy it removes the row (32 findings to 24 before the refresh).
- **docs**: `versioning.md` gains `generated_date` frontmatter (the schema pass required it).
- **Why**: `map-check` on main reported stale and false-positive findings that every 1.0 branch would meet again; the test-registration and fragment questions kept coming back from audits.
- **Tests**: `cargo test --test zoom_continuity -- --list` still lists the `zoom_land::` tests; `cargo fmt`, `clippy`, `cargo test --workspace` green.
