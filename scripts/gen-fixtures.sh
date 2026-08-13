#!/bin/sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
FIXTURES="$ROOT/openmix-core/tests/fixtures"
mkdir -p "$FIXTURES"
python3 "$ROOT/scripts/gen-fixtures.py"
afconvert -f flac -d flac -c 1 "$FIXTURES/sine1k_1s.wav" "$FIXTURES/sine1k_1s.flac"
echo "wrote $FIXTURES/sine1k_1s.flac"
if command -v swift >/dev/null 2>&1; then
    swift "$ROOT/scripts/gen-fixtures.swift" "$FIXTURES" \
        || afconvert -f mp3 -d .mp3 -c 1 "$FIXTURES/sine1k_1s.wav" "$FIXTURES/sine1k_1s.mp3" \
        || { echo "swift and afconvert mp3 encoding unavailable; falling back to lame" >&2; \
             command -v lame >/dev/null 2>&1 || brew install lame; \
             lame --preset standard "$FIXTURES/sine1k_1s.wav" "$FIXTURES/sine1k_1s.mp3"; }
else
    echo "swift not found; using lame" >&2
    command -v lame >/dev/null 2>&1 || brew install lame
    lame --preset standard "$FIXTURES/sine1k_1s.wav" "$FIXTURES/sine1k_1s.mp3"
fi
echo "fixtures written to $FIXTURES"