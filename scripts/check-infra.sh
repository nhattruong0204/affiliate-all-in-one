#!/usr/bin/env bash
# shellcheck disable=SC2016  # jq programs are deliberately single-quoted
# Static policy checks for the production stack (AAO-092). Needs `docker compose` and `jq`; no
# daemon, no network. Renders infra/compose.prod.yml with dummy values and fails on any violation of
# docs/INFRASTRUCTURE.md §2 "compose principles":
#
#   * every service: read-only rootfs, cap_drop ALL, no-new-privileges, memory limit, restart policy
#   * only caddy publishes 80/443; every other published port is bound to 127.0.0.1
#   * Postgres is reachable only from the `app` network, which is internal
#   * no secret-looking variable in `environment` unless it points at a *_FILE secret
#   * every image has an explicit tag, never `latest`
#   * web runs in production mode and trusts X-Forwarded-For (it sits behind caddy)
#
# Usage: scripts/check-infra.sh [compose-file]
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
file="${1:-$root/infra/compose.prod.yml}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

"$root/infra/bootstrap/gen-secrets.sh" "$tmp/secrets" >/dev/null
rendered="$tmp/compose.json"
AAO_SECRETS_DIR="$tmp/secrets" WEB_IMAGE=registry.invalid/aao-web:test SITE_ADDRESS=example.invalid \
  ACME_EMAIL=ops@example.invalid docker compose -f "$file" config --format json >"$rendered"

failures=0
fail() { echo "infra policy: $*" >&2; failures=$((failures + 1)); }

# jq prints one violation per line; each line becomes a failure.
check() { # description, jq program producing violation strings
  local desc=$1 program=$2 line
  while IFS= read -r line; do
    [[ -n "$line" ]] && fail "$desc: $line"
  done < <(jq -r "$program" "$rendered")
}

check "read_only must be true" \
  '.services | to_entries[] | select(.value.read_only != true) | .key'
check "cap_drop must contain ALL" \
  '.services | to_entries[] | select((.value.cap_drop // []) | index("ALL") | not) | .key'
check "security_opt must contain no-new-privileges" \
  '.services | to_entries[] | select((.value.security_opt // []) | map(startswith("no-new-privileges")) | any | not) | .key'
check "mem_limit must be set" \
  '.services | to_entries[] | select((.value.mem_limit // 0) == 0) | .key'
check "restart policy must be set" \
  '.services | to_entries[] | select((.value.restart // "") == "") | .key'

# Published ports: caddy may publish exactly 80 and 443; anyone else only on loopback.
check "only caddy may publish a public port, and only 80/443" \
  '.services | to_entries[] | .key as $svc | (.value.ports // [])[]
   | select(((.host_ip // "") != "127.0.0.1") and ($svc != "caddy" or ((.published | tostring) as $p | ($p != "80" and $p != "443"))))
   | "\($svc) publishes \(.published)"'
check "postgres/metrics/prometheus ports must never be published" \
  '.services | to_entries[] | .key as $svc | (.value.ports // [])[]
   | select((.target | tostring) as $t | ($t == "5432" or $t == "9100" or $t == "9090"))
   | "\($svc) publishes \(.target)"'

check "postgres must be on the app network only" \
  '.services.postgres.networks | keys | select(. != ["app"]) | tostring'
check "the app and obs networks must be internal" \
  '.networks | to_entries[] | select(.key == "app" or .key == "obs") | select(.value.internal != true) | .key'

check "secret-looking environment variables must be *_FILE" \
  '.services | to_entries[] | .key as $svc | (.value.environment // {}) | keys[]
   | select(test("PASSWORD|SECRET|TOKEN|PRIVATE|CREDENTIAL|(^|_)KEY(_|$)"; "i")) | select(test("_FILE$") | not)
   | "\($svc) sets \(.)"'

check "images must have an explicit tag and not be latest" \
  '.services | to_entries[] | select(.value.image | test("^[^@]+:[^:/@]+(@sha256:[0-9a-f]{64})?$|@sha256:[0-9a-f]{64}$") | not) | "\(.key) uses \(.value.image)"'
check "images must not use :latest" \
  '.services | to_entries[] | select(.value.image | test(":latest($|@)")) | .key'

check "web must run with APP_ENV=production and TRUST_FORWARDED_FOR=true" \
  '.services.web.environment | select(.APP_ENV != "production" or .TRUST_FORWARDED_FOR != "true") | "APP_ENV=\(.APP_ENV) TRUST_FORWARDED_FOR=\(.TRUST_FORWARDED_FOR)"'
check "web must read its database URL from a secret file" \
  '.services.web.environment | select((.DATABASE_URL_FILE // "") != "/run/secrets/database_url" or has("DATABASE_URL")) | "DATABASE_URL_FILE not set or inline DATABASE_URL present"'

if ((failures > 0)); then
  echo "infra policy: $failures violation(s) in $file" >&2
  exit 1
fi

# Digest pinning is enforced by Renovate PRs (docker:pinDigests); say what is still unpinned.
unpinned="$(jq -r '.services | to_entries[] | select(.value.image | contains("@sha256:") | not) | "\(.key)=\(.value.image)"' "$rendered" | paste -sd' ' -)"
[[ -z "$unpinned" ]] || echo "notice: images not yet pinned by digest: $unpinned"
echo "infra policy OK ($(jq '.services | length' "$rendered") services)"
