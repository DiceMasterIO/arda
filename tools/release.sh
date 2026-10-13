#!/usr/bin/env bash
# Release helper. See docs/capstone/07-operations.md "Release process".
#
#   tools/release.sh <version>          bump every version site, re-bless goldens
#   tools/release.sh --check [version]  verify every version site agrees (no writes)
#   tools/release.sh --notes <version>  print that version's CHANGELOG.md section
#
# Bumping leaves the CHANGELOG section, the commit and the tag to the
# maintainer and prints the exact commands.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

semver_re='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'

die() { echo "release.sh: $*" >&2; exit 1; }

workspace_version() {
    awk '/^\[workspace\.package\]/ {on=1; next} /^\[/ {on=0}
         on && /^version *=/ {gsub(/[" ]/, "", $0); sub(/^version=/, "", $0); print; exit}' Cargo.toml
}

# Prints "<site>: <version>" for every place a version lives.
list_sites() {
    echo "Cargo.toml [workspace.package]: $(workspace_version)"
    awk '/^\[workspace\.dependencies\]/ {on=1; next} /^\[/ {on=0}
         on && /path *= *"crates\// {
             name=$1
             match($0, /version *= *"[^"]*"/)
             v=substr($0, RSTART, RLENGTH); gsub(/version *= *|"/, "", v)
             print "Cargo.toml dep " name ": " v
         }' Cargo.toml
    # Workspace members in the lockfile are the entries with no `source`.
    awk '/^\[\[package\]\]/ {name=""; ver=""; src=0; next}
         /^name = / {gsub(/"/, "", $3); name=$3}
         /^version = / {gsub(/"/, "", $3); ver=$3}
         /^source = / {src=1}
         /^$/ {if (name ~ /^arda/ && !src && ver != "") print "Cargo.lock " name ": " ver; name=""}
         END {if (name ~ /^arda/ && !src && ver != "") print "Cargo.lock " name ": " ver}' Cargo.lock
    echo "apps/viewer/package.json: $(grep -m1 '"version"' apps/viewer/package.json | sed 's/.*: *"\([^"]*\)".*/\1/')"
    local n=0 v
    while IFS= read -r v; do
        n=$((n + 1))
        echo "apps/viewer/package-lock.json #$n: $v"
    done < <(grep -m2 '"version"' apps/viewer/package-lock.json | sed 's/.*: *"\([^"]*\)".*/\1/')
    echo "crates/arda-server/API.md arda_version: $(grep -m1 -o '"arda_version":"[^"]*"' crates/arda-server/API.md | sed 's/.*:"\(.*\)"/\1/')"
}

check() {
    local want="${1:-$(workspace_version)}"
    [ -n "$want" ] || die "cannot read the workspace version from Cargo.toml"
    local bad=0 sites=0 line
    while IFS= read -r line; do
        sites=$((sites + 1))
        if [ "${line##*: }" != "$want" ]; then
            echo "MISMATCH (want $want): $line" >&2
            bad=$((bad + 1))
        fi
    done < <(list_sites)
    # 21 path entries + the workspace version + lock members + viewer + API.md.
    [ "$sites" -ge 40 ] || die "found only $sites version sites; the layout changed, update tools/release.sh"
    [ "$bad" -eq 0 ] || die "$bad of $sites version sites disagree with $want"
    echo "release.sh --check: all $sites version sites are $want"
}

notes() {
    local v="${1:?--notes needs a version}"
    local out
    out="$(awk -v v="$v" '
        $0 ~ "^## " && !on { split($0, a, " "); if (a[2] == v) {on=1; next} }
        on && /^## / {exit}
        on {print}' CHANGELOG.md | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' | sed '/./,$!d')"
    [ -n "$out" ] || die "CHANGELOG.md has no '## $v' section with content"
    printf '%s\n' "$out"
}

bump() {
    local new="$1" old
    [[ "$new" =~ $semver_re ]] || die "'$new' is not a semantic version"
    old="$(workspace_version)"
    [ "$old" != "$new" ] || die "workspace is already at $new"
    [ -z "$(git status --porcelain --untracked-files=no)" ] || die "tracked files are modified; commit or stash first"

    # Workspace version, then every internal path entry.
    sed -i -E '/^\[workspace\.package\]/,/^\[/ s/^version *= *"[^"]*"/version = "'"$new"'"/' Cargo.toml
    sed -i -E '/path *= *"crates\// s/version *= *"[^"]*"/version = "'"$new"'"/' Cargo.toml
    sed -i -E '0,/"arda_version":"[^"]*"/ s/"arda_version":"[^"]*"/"arda_version":"'"$new"'"/' crates/arda-server/API.md
    (cd apps/viewer && npm version "$new" --no-git-tag-version --allow-same-version >/dev/null)
    cargo update -w
    ARDA_BLESS=1 cargo test --release --test golden_world --test golden_recipes
    # Blessing rewrites every golden without a trailing newline. Keep only the
    # files whose content really changed (the recipe-5/6 pins must not move).
    local g
    for g in tests/golden/micro-42*.txt; do
        if git diff --quiet --ignore-all-space -- "$g"; then git checkout -- "$g"; fi
    done

    check "$new"
    expected_diff
    cat <<MSG

Bumped $old -> $new. Next, by hand:
  1. Add a '## $new — $(date +%F)' section to CHANGELOG.md (tools/release.sh --notes $new previews it).
  2. git add Cargo.toml Cargo.lock apps/viewer/package.json apps/viewer/package-lock.json \\
         crates/arda-server/API.md tests/golden CHANGELOG.md
  3. git commit -m 'release: Arda v$new'
  4. git tag v$new
  5. Push the branch and the tag only when ready: the tag starts .github/workflows/release.yml.
MSG
}

# git diff must touch only files a release is allowed to change.
expected_diff() {
    local f bad=0
    while IFS= read -r f; do
        case "$f" in
            Cargo.toml|Cargo.lock|apps/viewer/package.json|apps/viewer/package-lock.json|\
crates/arda-server/API.md|tests/golden/micro-42*.txt) ;;
            *) echo "unexpected change: $f" >&2; bad=1 ;;
        esac
    done < <(git diff --name-only)
    [ "$bad" -eq 0 ] || die "the bump changed files outside the release set"
    echo "release.sh: git diff touches only release files"
}

case "${1:-}" in
    ""|-h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; [ -n "${1:-}" ] || exit 2 ;;
    --check) check "${2:-}" ;;
    --notes) notes "${2:-}" ;;
    -*) die "unknown option $1" ;;
    *) bump "$1" ;;
esac
