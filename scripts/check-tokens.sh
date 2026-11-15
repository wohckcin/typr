#!/usr/bin/env bash
# Refuses a spacing, size or type literal in a screen.
#
# Every dimension a screen draws with comes from `crates/typr/src/design`, so a literal here means
# a screen invented a number of its own and the token set no longer describes the app. Font sizes
# and line heights are refused the same way: a type style is a token (`type_body`, `type_input`,
# `type_editor`, `type_caption`), never a size written at the call site.
#
# Usage: scripts/check-tokens.sh [file...]
#   No arguments checks the screens this change covers. A file named on the command line is
#   checked in place, which is how the test proves the check refuses one.
set -uo pipefail

screens=(
  crates/typr/src/library/mod.rs
  crates/typr/src/library/tile/mod.rs
  crates/typr/src/editor/mod.rs
  crates/typr/src/study/mod.rs
  crates/typr/src/history/mod.rs
  crates/typr/src/settings.rs
)

if [ "$#" -gt 0 ]; then
  screens=("$@")
fi

# A dimension setter followed by a digit: `.px(24.0)`, `.h(30.0)`, `.radius(6.0)`, `.lh(20.0)`.
pattern='\.(px|py|pl|pr|pt|pb|padding|gap|w|h|max_w|max_h|min_w|min_h|radius|lh|font_size)\([0-9]'

status=0
for file in "${screens[@]}"; do
  if [ ! -f "$file" ]; then
    echo "check-tokens: $file is missing" >&2
    status=1
    continue
  fi
  if hits=$(grep -nE "$pattern" "$file"); then
    echo "check-tokens: $file writes a dimension as a literal; use crates/typr/src/design:" >&2
    printf '%s\n' "$hits" | sed "s|^|  $file:|" >&2
    status=1
  fi
done

exit "$status"
