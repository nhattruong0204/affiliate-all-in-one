#!/usr/bin/env bash
# INTERIM logical backup loop (AAO-092). Replaced by AAO-093 (WAL archiving, encryption, off-site).
#
# Writes <BACKUP_DIR>/aao-<UTC timestamp>.dump (pg_dump custom format) every
# BACKUP_INTERVAL_SECONDS, keeps BACKUP_RETENTION_DAYS days, and records the time of the last
# success in <BACKUP_DIR>/.last_success (epoch seconds) for a future "backup age" alert.
# BACKUP_ONCE=1 takes one backup and exits (used by the CI smoke test).
#
# Required environment: PGHOST, PGUSER, PGDATABASE, PGPASSWORD_FILE. The password is read from the
# secret file; it is never an environment variable of the container.
set -euo pipefail

: "${PGHOST:?}" "${PGUSER:?}" "${PGDATABASE:?}" "${PGPASSWORD_FILE:?}"
dir="${BACKUP_DIR:-/backups}"
interval="${BACKUP_INTERVAL_SECONDS:-86400}"
retention="${BACKUP_RETENTION_DAYS:-7}"

[[ "$interval" =~ ^[0-9]+$ && "$interval" -ge 60 ]] || { echo "BACKUP_INTERVAL_SECONDS must be an integer >= 60" >&2; exit 2; }
[[ "$retention" =~ ^[0-9]+$ && "$retention" -ge 1 ]] || { echo "BACKUP_RETENTION_DAYS must be an integer >= 1" >&2; exit 2; }

PGPASSWORD="$(<"$PGPASSWORD_FILE")"
export PGPASSWORD
umask 077
mkdir -p "$dir"

log() { printf '{"ts":"%s","service":"backup","msg":"%s"}\n' "$(date -u +%FT%TZ)" "$*"; }

backup_once() {
  local stamp target partial
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  target="$dir/aao-${stamp}.dump"
  partial="${target}.partial"
  if pg_dump --format=custom --no-owner --file="$partial"; then
    mv "$partial" "$target"
    date +%s >"$dir/.last_success"
    log "backup written: $(basename "$target") $(stat -c %s "$target") bytes"
    # Prune only after a successful dump, so a failing database never empties the directory.
    find "$dir" -maxdepth 1 -name 'aao-*.dump' -mtime "+${retention}" -print -delete \
      | while read -r old; do log "pruned: $(basename "$old")"; done
    return 0
  fi
  rm -f "$partial"
  log "backup FAILED"
  return 1
}

if [[ "${BACKUP_ONCE:-0}" == "1" ]]; then
  backup_once
  exit $?
fi

while true; do
  backup_once || true
  sleep "$interval"
done
