#!/usr/bin/env bash
# Smoke test for built release binaries.
#
#   tools/smoke-binaries.sh <bin-dir> [--serve]
#
# Always checks `arda --version` and `arda-server --version`. With --serve it
# also generates the micro world for seed 42, starts arda-server on it and
# requires /v1/health to answer, then stops the server.
set -euo pipefail

bindir="${1:?usage: $0 <bin-dir> [--serve]}"
serve="${2:-}"
ext=""
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) ext=".exe" ;; esac
arda="$bindir/arda$ext"
server="$bindir/arda-server$ext"

"$arda" --version | grep -q '^arda ' || { echo "FAIL: arda --version" >&2; exit 1; }
"$server" --version | grep -q '^arda-server ' || { echo "FAIL: arda-server --version" >&2; exit 1; }
echo "versions ok: $("$arda" --version), $("$server" --version)"
[ "$serve" = --serve ] || exit 0

work="$(mktemp -d)"
pid=""
# shellcheck disable=SC2329 # invoked via trap
cleanup() {
    [ -z "$pid" ] || kill "$pid" 2>/dev/null || true
    rm -rf "$work"
}
trap cleanup EXIT

"$arda" generate --seed 42 --micro --out "$work/world" >/dev/null
port="${ARDA_SMOKE_PORT:-18788}"
"$server" --world "$work/world" --port "$port" >"$work/server.log" 2>&1 &
pid=$!
for _ in $(seq 1 60); do
    if curl -fsS "http://127.0.0.1:$port/v1/health" >"$work/health.json" 2>/dev/null; then
        grep -q '"status":"ok"' "$work/health.json" || { echo "FAIL: unexpected health body" >&2; exit 1; }
        grep -q '"seed":"42"' "$work/health.json" || { echo "FAIL: health does not report seed 42" >&2; exit 1; }
        echo "serve ok: $(cat "$work/health.json")"
        exit 0
    fi
    kill -0 "$pid" 2>/dev/null || { echo "FAIL: server exited" >&2; tail -20 "$work/server.log" >&2; exit 1; }
    sleep 1
done
echo "FAIL: /v1/health never answered" >&2
tail -20 "$work/server.log" >&2
exit 1
