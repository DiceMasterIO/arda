#!/usr/bin/env bash
# Builds the arda image and checks it end to end against a stored world.
#
#   tools/docker-smoke.sh <world-dir> [--ai <art-gen-lib-dir>]
#
# Without --ai: the placeholder-only image. With --ai: the image built with
# `--build-context artlib=<dir>`, which must serve the AI library on top.
set -euo pipefail

world=""
ai=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --ai) ai="${2:?--ai needs a directory}"; shift 2 ;;
        *) world="$1"; shift ;;
    esac
done
[ -n "$world" ] || { echo "usage: $0 <world-dir> [--ai <lib-dir>]" >&2; exit 2; }
world="$(cd "$world" && pwd)"
root="$(cd "$(dirname "$0")/.." && pwd)"

variant=placeholder
tag=arda:smoke-placeholder
build_args=(--build-arg "REVISION=$(git -C "$root" rev-parse HEAD)")
if [ -n "$ai" ]; then
    variant=ai
    tag=arda:smoke-ai
    build_args+=(--build-context "artlib=$(cd "$ai" && pwd)")
fi
name="arda-smoke-$variant-$$"
port="${ARDA_SMOKE_PORT:-18787}"

cleanup() { docker rm -f "$name" >/dev/null 2>&1 || true; }
trap cleanup EXIT
fail() { echo "FAIL: $*" >&2; docker logs "$name" 2>&1 | tail -20 >&2 || true; exit 1; }

echo "== build ($variant)"
docker build "${build_args[@]}" -t "$tag" "$root"

echo "== run"
cleanup
docker run -d --name "$name" --read-only --tmpfs /tmp -v "$world:/worlds/world:ro" \
    -p "127.0.0.1:$port:8787" "$tag" >/dev/null

echo "== wait for health"
for _ in $(seq 1 60); do
    status="$(docker inspect -f '{{.State.Health.Status}}' "$name")"
    [ "$status" = healthy ] && break
    [ "$(docker inspect -f '{{.State.Running}}' "$name")" = true ] || fail "container exited"
    sleep 1
done
[ "$status" = healthy ] || fail "never healthy (last status: $status)"
base="http://127.0.0.1:$port/v1"

echo "== endpoints"
curl -fsS "$base/health" >/dev/null || fail "/v1/health"
curl -fsS "$base/world" | grep -q '"seed"' || fail "/v1/world has no seed"
lib="$(curl -fsS "$base/tactical/library")"
echo "$lib" | grep -q '"assets"' || fail "/v1/tactical/library has no assets"
logs="$(docker logs "$name" 2>&1)"
# The banner prints the stacked library_version, top layer first; read each
# layer's version from its own catalog so a library bump needs no edit here.
lib_version() {
    sed -n 's/.*"library_version": *"\([^"]*\)".*/\1/p' "$1/catalog.json" | head -n 1
}
placeholder_version="$(lib_version "$root/assets/tactical/placeholder")"
[ -n "$placeholder_version" ] || fail "no library_version in the placeholder catalog"
if [ "$variant" = ai ]; then
    ai_version="$(lib_version "$ai")"
    [ -n "$ai_version" ] || fail "no library_version in $ai/catalog.json"
    want="tactical library $ai_version:$placeholder_version,"
else
    want="tactical library $placeholder_version,"
fi
echo "$logs" | grep -qF "$want" || fail "expected banner '$want'"
png="$(mktemp)"
curl -fsS "$base/tactical/cell/10/10.png?ppsq=16" -o "$png" || fail "tactical cell PNG"
[ "$(head -c 8 "$png" | od -An -tx1 | tr -d ' \n')" = 89504e470d0a1a0a ] || fail "not a PNG"
rm -f "$png"

echo "== user, port, cli"
[ "$(docker exec "$name" id -u)" = 10001 ] || fail "not running as 10001"
[ "$(docker inspect -f '{{.Config.User}}' "$tag")" = arda ] || fail "image user is not arda"
docker inspect -f '{{json .Config.ExposedPorts}}' "$tag" | grep -q 8787 || fail "8787 not exposed"
docker run --rm --entrypoint arda "$tag" --version | grep -q '^arda' || fail "arda --version"

echo "== docker stop under 3 s (SIGTERM)"
start="$(date +%s%N)"
docker stop "$name" >/dev/null
elapsed_ms=$(( ($(date +%s%N) - start) / 1000000 ))
echo "stopped in ${elapsed_ms} ms"
[ "$elapsed_ms" -lt 3000 ] || fail "docker stop took ${elapsed_ms} ms"
[ "$(docker inspect -f '{{.State.ExitCode}}' "$name")" = 0 ] || fail "non-zero exit after SIGTERM"

echo "OK ($variant)"
