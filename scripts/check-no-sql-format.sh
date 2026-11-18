#!/usr/bin/env bash
# Forbid building SQL with format!/write!/writeln! (AGENTS.md §4: "No string-built SQL").
# Use sqlx::query!/query_as! (compile-time checked) or sqlx::QueryBuilder::push_bind.
#
# This is a guard-rail heuristic, not a parser: it flags a format-family macro whose first
# string literal contains SQL statement shape. Review remains the primary control.
# Usage: check-no-sql-format.sh [dir ...]   (default: crates)
set -euo pipefail
cd "$(dirname "$0")/.."
dirs=("$@"); [[ ${#dirs[@]} -eq 0 ]] && dirs=(crates)

export SQL_FORMAT_RE='\b(?:format|write|writeln|format_args)!\s*\(\s*(?:[A-Za-z_][A-Za-z0-9_.]*\s*,\s*)?r?#*"[^"]*?\b(?:select\s[^"]*?\sfrom|insert\s+into|update\s+[A-Za-z_"\.]+\s+set|delete\s+from)\b'

status=0
while IFS= read -r -d '' file; do
  if perl -0777 -ne '
    my $found = 0;
    while (/$ENV{SQL_FORMAT_RE}/gis) {
      my $line = 1 + (substr($_, 0, $-[0]) =~ tr/\n//);
      print "$ARGV:$line: SQL built with a formatting macro; use sqlx::query! or QueryBuilder::push_bind\n";
      $found = 1;
    }
    exit($found ? 1 : 0);
  ' "$file"; then :; else status=1; fi
done < <(find "${dirs[@]}" -name '*.rs' -not -path '*/target/*' -print0)

if [[ $status -ne 0 ]]; then
  echo "SQL-format check FAILED" >&2
  exit 1
fi
echo "SQL-format check OK"
