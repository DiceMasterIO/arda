# 2026-10-12 — arda-server container image

- **Docker**: the `Dockerfile` builds `arda` and `arda-server` from a pinned `rust:1.98.1-bookworm` with `RUSTUP_TOOLCHAIN` set and BuildKit cache mounts, into a digest-pinned `debian:bookworm-slim` as user/group 10001. It adds `EXPOSE 8787`, a `HEALTHCHECK`, OCI labels, `WORKDIR /opt/arda` and a `.dockerignore` (the build context drops from several GB to a few MB). Entry `arda-entrypoint` (`tools/docker-entrypoint.sh`) serves `/worlds/world` on `0.0.0.0:8787`; `--entrypoint arda` runs the CLI.
- **Library variants**: the placeholder library ships always; the AI library enters only through the optional BuildKit context `artlib` and is stacked over the placeholder. Maintainer decision D1: the published image is the placeholder variant only.
- **arda-server**: SIGTERM now shuts the server down like Ctrl-C (Unix); new `--probe` flag backed by a std-only `probe` module for the healthcheck.
- **Tooling**: `tools/docker-smoke.sh` builds either variant and checks health, world, library, a tactical PNG, uid 10001, the CLI entry and a sub-3 s `docker stop`.
- **Docs**: README Docker section (the stale `arda serve` line is gone), 07-operations Processes, flags and Infrastructure rows, API.md, OI-30.
- **Why**: nobody could deploy the HTTP service from the image, and `docker stop` hung ten seconds on SIGTERM.
- **Tests**: probe unit tests (200, non-200, refused, silent close); `tests/serve.rs` shutdown-future test; the smoke script for both variants.
- **Open**: the `commercial_licence` validator rule from logic/11 is not implemented, so `LicenseRef-AI-generated` is not gated.
