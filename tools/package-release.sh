#!/usr/bin/env bash
# Packages the release binaries for one target.
#
#   tools/package-release.sh <version> <target> <bin-dir> <out-dir>
#
# Writes <out-dir>/arda-<version>-<target>.tar.gz (.zip for *-windows-*) holding
# arda, arda-server, LICENSE, NOTICE, README.md and CHANGELOG.md under one
# top-level directory, plus a .sha256 file beside it.
set -euo pipefail

version="${1:?usage: $0 <version> <target> <bin-dir> <out-dir>}"
target="${2:?missing target}"
bindir="${3:?missing bin dir}"
outdir="${4:?missing out dir}"
root="$(cd "$(dirname "$0")/.." && pwd)"

ext=""
case "$target" in *-windows-*) ext=".exe" ;; esac

name="arda-$version-$target"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/$name" "$outdir"
outdir="$(cd "$outdir" && pwd)"
for bin in arda arda-server; do
    install -m 755 "$bindir/$bin$ext" "$stage/$name/$bin$ext"
done
for doc in LICENSE NOTICE README.md CHANGELOG.md; do
    install -m 644 "$root/$doc" "$stage/$name/$doc"
done

if [ -n "$ext" ]; then
    archive="$name.zip"
    rm -f "$outdir/$archive"
    (cd "$stage" && 7z a -tzip -bd "$outdir/$archive" "$name" >/dev/null)
else
    archive="$name.tar.gz"
    (cd "$stage" && tar -czf "$outdir/$archive" "$name")
fi

if command -v sha256sum >/dev/null; then
    (cd "$outdir" && sha256sum "$archive" > "$archive.sha256")
else
    (cd "$outdir" && shasum -a 256 "$archive" > "$archive.sha256")
fi
echo "$outdir/$archive"
