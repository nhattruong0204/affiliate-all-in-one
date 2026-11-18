#!/usr/bin/env bash
# sqlx offline-metadata gate. Fails closed: once any crate uses sqlx, this script must be
# replaced by the real check (AAO-004 wires `cargo sqlx prepare --check` against a CI Postgres).
set -euo pipefail
cd "$(dirname "$0")/.."

users=$(cargo metadata --format-version 1 --no-deps \
  | jq -r '[.packages[] | select(any(.dependencies[]; .name == "sqlx")) | .name] | join(" ")')

if [[ -z "$users" ]]; then
  echo "sqlx check skipped: no crate depends on sqlx yet (enabled by AAO-004)"
  exit 0
fi
echo "ERROR: sqlx is used by [$users] but scripts/sqlx-check.sh does not verify .sqlx metadata yet." >&2
echo "Replace this stub with 'cargo sqlx prepare --workspace --check' in the same PR that adds sqlx." >&2
exit 1
