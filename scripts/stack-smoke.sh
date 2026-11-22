#!/usr/bin/env bash
# Brings the whole production stack up from scratch on this machine and checks it end to end
# (AAO-092 "staging smoke"). Uses plain HTTP (SITE_ADDRESS=:80) so no certificate is needed; TLS is
# Caddy's automatic HTTPS and is only exercised on a real domain.
#
# Usage: scripts/stack-smoke.sh <web-image>     (e.g. aao-web:ci)
#
# Needs docker + compose, curl, and free host ports 80, 443 and 3000. Tears everything down
# (including volumes) on exit.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
web_image="${1:?usage: stack-smoke.sh <web-image>}"
tmp="$(mktemp -d)"
project="aao-smoke"
export AAO_SECRETS_DIR="$tmp/secrets"
export WEB_IMAGE="$web_image" SITE_ADDRESS=":80" ACME_EMAIL="ops@example.invalid"
# A low page limit so the spoofed-header check below can reach it quickly.
export RATE_LIMIT_PAGES_PER_MINUTE=5
compose=(docker compose -p "$project" -f "$root/infra/compose.prod.yml")

fail() { echo "FAIL: $*" >&2; exit 1; }
cleanup() {
  local status=$?
  if ((status != 0)); then "${compose[@]}" logs --no-color --tail 60 >&2 || true; fi
  "${compose[@]}" down --volumes --remove-orphans >/dev/null 2>&1 || true
  rm -rf "$tmp"
  exit "$status"
}
trap cleanup EXIT

http_code() { curl -s -o /dev/null -w '%{http_code}' "$@"; }
wait_for() { # description, attempts, command...
  local desc=$1 attempts=$2; shift 2
  for _ in $(seq 1 "$attempts"); do "$@" >/dev/null 2>&1 && return 0; sleep 2; done
  fail "timed out waiting for: $desc"
}

echo "== up from scratch"
"$root/infra/bootstrap/gen-secrets.sh" "$AAO_SECRETS_DIR" >/dev/null
"${compose[@]}" up --detach --wait --wait-timeout 240

echo "== public surface through caddy"
base="http://127.0.0.1"
[[ "$(http_code "$base/")" == "200" ]] || fail "/ did not return 200"
[[ "$(http_code "$base/healthz")" == "200" ]] || fail "/healthz did not return 200"
[[ "$(http_code "$base/readyz")" == "200" ]] || fail "/readyz did not return 200 (database reachable?)"
[[ "$(http_code "$base/metrics")" == "404" ]] || fail "/metrics must not be reachable through caddy"

headers="$(curl -s -D - -o /dev/null "$base/")"
for name in content-security-policy strict-transport-security x-content-type-options x-frame-options referrer-policy; do
  count="$(grep -ci "^${name}:" <<<"$headers" || true)"
  [[ "$count" == "1" ]] || fail "expected exactly one '${name}' header from web (caddy must not add its own), got ${count}"
done

echo "== the rate limiter keys on the real client, not on a client-supplied X-Forwarded-For"
limited=0
for i in $(seq 1 12); do
  code="$(http_code -H "X-Forwarded-For: 203.0.113.${i}" "$base/en")"
  [[ "$code" == "429" ]] && limited=1
done
((limited == 1)) || fail "12 requests with spoofed X-Forwarded-For were never limited (proxy contract broken)"
# Probes and assets stay reachable while pages are limited.
[[ "$(http_code "$base/healthz")" == "200" ]] || fail "/healthz must be exempt from the page limit"

echo "== nothing but 80/443 is published beyond loopback"
published="$(docker ps --filter "label=com.docker.compose.project=${project}" --format '{{.Names}} {{.Ports}}')"
if grep -Eq '0\.0\.0\.0:(5432|9090|9100|3000)->' <<<"$published"; then
  echo "$published" >&2; fail "an internal service is published on all interfaces"
fi

echo "== prometheus scrapes web"
scraped() { "${compose[@]}" exec -T prometheus promtool query instant http://127.0.0.1:9090 'up{job="web"}' | grep -q '=> 1 @'; }
wait_for "prometheus to scrape web" 40 scraped
rules="$("${compose[@]}" exec -T prometheus promtool query instant http://127.0.0.1:9090 'count(ALERTS or vector(0))' || true)"
[[ -n "$rules" ]] || fail "prometheus does not answer queries"

echo "== grafana has the provisioned dashboard"
grafana_pw="$(<"$AAO_SECRETS_DIR/grafana_admin_password")"
dash() { [[ "$(http_code -u "admin:${grafana_pw}" http://127.0.0.1:3000/api/dashboards/uid/aao-red)" == "200" ]]; }
wait_for "grafana dashboard aao-red" 30 dash
[[ "$(http_code http://127.0.0.1:3000/api/dashboards/uid/aao-red)" == "401" ]] || fail "grafana must require login"

echo "== backup wrote a restorable dump"
have_dump() { "${compose[@]}" exec -T backup bash -c 'ls /backups/aao-*.dump >/dev/null && test -s /backups/.last_success'; }
wait_for "the first backup" 30 have_dump
"${compose[@]}" exec -T backup bash -c 'pg_restore --list /backups/aao-*.dump >/dev/null' \
  || fail "the dump is not a valid pg_dump archive"

echo "stack smoke OK"
