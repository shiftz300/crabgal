#!/usr/bin/env bash
set -euo pipefail

project="${1:-projects/test-project}"
output="${2:-target/release-package}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
source "$root/dev/scripts/lib/audio-features.sh"

cd "$root"
require_project_directory "$project"
if [[ -z "${CRABGAL_HEXZ_PASSWORD:-}" ]]; then
    echo "CRABGAL_HEXZ_PASSWORD must be set" >&2
    exit 2
fi
if ! command -v hexz >/dev/null 2>&1; then
    echo "hexz CLI is required; install maincoretech/hexz_k with its cli feature" >&2
    exit 2
fi

case "$output" in
    target/*) ;;
    *)
        echo "release output must stay under target/: $output" >&2
        exit 2
        ;;
esac

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
mkdir -p "$staging/project"
cp -R "$project"/. "$staging/project/"

# Runtime state and generated caches must never enter the encrypted artifact.
find "$staging/project" -type d \( -name saves -o -name imported_assets \) -prune \
    -exec rm -rf {} +
find "$staging/project" -type f \( -name '.DS_Store' -o -name '*.meta' \) -delete

rm -rf "$output"
mkdir -p "$output"
cp "$root/assets/icons/crabgal-256.png" "$output/crabgal.png"
HEXZ_PASSWORD="$CRABGAL_HEXZ_PASSWORD" \
    hexz pack "$staging/project" "$output/game.hxz" \
    --compression zstd --encrypt --block-size 65536

CRABGAL_HEXZ_PASSWORD="$CRABGAL_HEXZ_PASSWORD" \
    build_engine_for_project "$staging/project" --release --locked
if [[ -f target/release/crabgal.exe ]]; then
    cp target/release/crabgal.exe "$output/crabgal.exe"
    cat > "$output/run.bat" <<'BAT'
@echo off
"%~dp0crabgal.exe" "%~dp0game.hxz"
BAT
else
    cp target/release/crabgal "$output/crabgal"
    cat > "$output/run.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")" && pwd)"
exec "$root/crabgal" "$root/game.hxz"
SH
    chmod +x "$output/crabgal" "$output/run.sh"
fi

printf '%s\n' "$root/$output"
