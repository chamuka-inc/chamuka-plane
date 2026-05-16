#!/usr/bin/env bash
set -euo pipefail

echo "=== Sandbox Platform Status ==="
echo

head_info=$(git log -1 --pretty=format:"%h — %s" 2>/dev/null || echo "(no commits)")
echo "HEAD: $head_info"
echo

body=$(git log -1 --pretty=format:"%B" 2>/dev/null || true)
phase=$(echo "$body" | grep "^Phase:" | head -1 || true)
next=$(echo "$body" | sed -n '/^Next:/,/^[A-Z][a-z]*:\|^$/p' | sed '$d' || true)
blocked=$(echo "$body" | grep "^Blocked:" | head -1 || true)

if [ -n "$phase" ]; then
    echo "$phase"
else
    echo "Phase: (none)"
fi

if [ -n "$next" ]; then
    echo "$next"
else
    echo "Next: (none)"
fi

if [ -n "$blocked" ]; then
    echo "$blocked"
fi

echo

if git diff --quiet 2>/dev/null && git diff --cached --quiet 2>/dev/null; then
    echo "Tree state: clean"
else
    echo "Tree state: dirty"
fi

echo
echo "Recent phase tags:"
tags=$(git tag --sort=-creatordate -l "phase-*" 2>/dev/null | head -5)
if [ -n "$tags" ]; then
    echo "$tags" | while read -r tag; do
        date=$(git log -1 --format="%ar" "$tag" 2>/dev/null || echo "unknown")
        echo "  $tag    ($date)"
    done
else
    echo "  (none)"
fi
