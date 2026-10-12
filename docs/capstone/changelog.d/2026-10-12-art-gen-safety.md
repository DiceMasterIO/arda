# Art generation spend caps and no double billing

- **Caps**: `tools/art-gen/generate.mjs` takes `--max-images N` and `--max-spend USD` (default `$ART_GEN_MAX_SPEND`). A run whose worst case (images x price x (1 + reviewer remakes)) exceeds `--max-spend` is refused with exit 2 before any request, also under `--dry-run`. `--yes` with an unknown price needs `--max-images` or `--price-per-image`. The summary prints the worst case.
- **Run-time guard**: every submission (new project, re-sent batch, re-posted failed or cancelled item) counts; the run stops cleanly at the cap with state and `import.toml` written and exit 3.
- **Dropped POST**: parallel mode writes a `posting` intent before each `POST /api/projects`; on a network error it reads `GET /api/projects` and adopts a project created meanwhile instead of posting again, retries once only when the list shows none, and exits 1 when the list is unreadable. Leftover intents are reconciled at the next start.
- **Code**: new `tools/art-gen/safety.mjs`; the stand-in server moved to `tools/art-gen/test/fake-slopify.mjs` with project listing and connection-drop switches; `tools/art-gen/test/safety.test.mjs` adds the cap and drop cases.
- **Docs**: `logic/20-art-generation.md` (guard, estimate, parallel, state, exit, unhappy paths, invariants), `tools/art-gen/README.md` (Spending safety), `tools/art-gen/plan.md`.
