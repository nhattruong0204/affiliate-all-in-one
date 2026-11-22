#!/usr/bin/env bash
# Creates the file-backed secrets that infra/compose.prod.yml mounts under /run/secrets (AAO-092).
#
#   infra/bootstrap/gen-secrets.sh [secrets-dir]      # default: infra/secrets (gitignored)
#
# Idempotent and non-destructive: existing files are never overwritten (rotating a database
# password also means changing it inside Postgres, which is a runbook step, RB-09). Values are
# never printed. The directory is 0700 and the files 0444: compose bind-mounts them into
# containers that run as other UIDs (web is 10001, postgres 999), so the files must be readable
# by them, and the directory permission is what keeps other host users out.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
dir="${1:-${AAO_SECRETS_DIR:-$root/infra/secrets}}"

random_hex() { od -An -N"$1" -tx1 /dev/urandom | tr -d ' \n'; }

mkdir -p "$dir"
chmod 700 "$dir"
umask 077

write_secret() { # name value
  local file="$dir/$1"
  if [[ -e "$file" ]]; then
    echo "kept    $file"
    return
  fi
  printf '%s' "$2" >"$file"
  chmod 444 "$file"
  echo "created $file"
}

db_password_file="$dir/postgres_password"
if [[ -e "$db_password_file" ]]; then
  db_password="$(<"$db_password_file")"
else
  db_password="$(random_hex 24)"
fi

write_secret postgres_password "$db_password"
# Hex only, so the password needs no URL escaping.
write_secret database_url "postgres://aao:${db_password}@postgres:5432/aao"
write_secret grafana_admin_password "$(random_hex 18)"

echo "secrets are in $dir (read the Grafana admin password with: cat $dir/grafana_admin_password)"
