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
  crates/typr/src/shell.rs
  crates/typr/src/screens/sidebar.rs
  crates/typr/src/screens/grid.rs
  crates/typr/src/screens/editor.rs
  crates/typr/src/screens/history.rs
)

if [ "$#" -gt 0 ]; then
  screens=("$@")
fi

# A pixel length or a colour written as a value: `px(24.0)`, `rgb(0x1e1e1e)`, `hsla(..)`.
literal='\b(px|rgb|rgba|hsl|hsla)\([0-9]|\b(rgb|rgba|hsl|hsla)\(0x'
# Any `nsh_base` import other than the theme, and any call to a base control by name.
base='use nsh_base::|nsh_base::(button|input|checkbox|switch|select|tab|tabs|menu|popover|tooltip|dialog|alert_dialog|sheet|toast|toast_stack|radio_group|text_view|editor)\b'

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
