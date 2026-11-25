#!/usr/bin/env bash
# Refuses a literal or a base control in a screen.
#
# A screen draws with the tokens in `crates/typr/src/ui` (colours by role, radii, type and spacing
# from the Tailwind scale), and with the app's own components, never with a `nsh_base` control or a
# number of its own. The `ui` layer is where those values are named, so it is not checked here.
#
# Usage: scripts/check-tokens.sh [file...]
#   No arguments checks the screens this change covers. A file named on the command line is
#   checked in place, which is how the test proves the check refuses one.
set -uo pipefail

screens=(
  crates/typr/src/shell/mod.rs
  crates/typr/src/shell/screen.rs
  crates/typr/src/shell/store_bridge.rs
  crates/typr/src/shell/navigation.rs
  crates/typr/src/shell/study.rs
  crates/typr/src/shell/editor.rs
  crates/typr/src/screens/sidebar.rs
  crates/typr/src/screens/grid.rs
  crates/typr/src/screens/library.rs
  crates/typr/src/screens/editor/mod.rs
  crates/typr/src/screens/editor/events.rs
  crates/typr/src/screens/editor/diagram.rs
  crates/typr/src/screens/editor/dialogs.rs
  crates/typr/src/screens/editor/fields.rs
  crates/typr/src/screens/embed.rs
  crates/typr/src/screens/diagnostics.rs
  crates/typr/src/screens/history.rs
  crates/typr/src/screens/keymap.rs
  crates/typr/src/screens/settings.rs
  crates/typr/src/screens/study.rs
  crates/typr/src/screens/trash.rs
)

if [ "$#" -gt 0 ]; then
  screens=("$@")
fi

# Any pixel length or colour constructor written directly in a screen (`px(..)`, `rgb(..)`, `hsla(..)`).
literal='\b(px|rgb|rgba|hsl|hsla)\('
# Any `nsh_base` import or qualified reference other than `use nsh_base::(theme|tokens_of)`.
base='use nsh_base::|nsh_base::'

status=0
for file in "${screens[@]}"; do
  if [ ! -f "$file" ]; then
    echo "check-tokens: $file is missing" >&2
    status=1
    continue
  fi
  if hits=$(grep -nE "$literal" "$file"); then
    echo "check-tokens: $file writes a literal; take it from crates/typr/src/ui:" >&2
    printf '%s\n' "$hits" | sed "s|^|  $file:|" >&2
    status=1
  fi
  if hits=$(grep -nE "$base" "$file" | grep -vE 'use nsh_base::(theme|tokens_of)' ); then
    if [ -n "$hits" ]; then
      echo "check-tokens: $file uses a base control; use the app's ui component:" >&2
      printf '%s\n' "$hits" | sed "s|^|  $file:|" >&2
      status=1
    fi
  fi
done

exit "$status"
