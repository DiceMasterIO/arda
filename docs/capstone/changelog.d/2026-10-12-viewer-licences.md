# 2026-10-12 — Viewer npm licence policy, CI job and bundle notices

- **Viewer**: `apps/viewer/licence-policy.json` (runtime and build-only allow-lists, reasoned exceptions), `scripts/check-licences.mjs` (stdlib only, reads the lockfile, deterministic sorted output) and `npm run licences`. The Vite plugin `scripts/licence-notices.mjs` writes `THIRD_PARTY_LICENSES.txt` into `dist/` with Leaflet's BSD-2-Clause notice and the MIT texts for React, React DOM and Scheduler; the build fails if a runtime package has no licence file. `npm test` now also runs `node --test scripts/`.
- **CI**: new `viewer` job in `.github/workflows/ci.yml` (Node 26, SHA-pinned `actions/setup-node`): `npm ci`, licences, typecheck, lint, test, build. `actionlint` clean.
- **deny.toml**: BSD-2-Clause and ISC added to the Rust allow-list so one rule covers the repo (maintainer decision D7); no crate uses them today.
- **Docs**: 05-dependencies "npm license policy", standards vetting bar, 06-testing CI matrix, 07-operations developer workflow, viewer README scripts.
- **Why**: `deny.toml` covered Rust only and the viewer's BSD-2-Clause dependency requires its notice in redistributed bundles; Arda ships in a commercial product.
- **Tests**: six `node --test` cases on fixture lockfiles (runtime GPL fails, dev MPL passes, runtime MPL fails, missing licence fails, `OR`/`AND`, exception, sorting); real `npm ci`, `licences`, `typecheck`, `lint`, `test` (86 Vitest) and `build` green. `cargo deny` was not run (not installed).
