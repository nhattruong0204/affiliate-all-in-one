#!/usr/bin/env bash
# Verifies a built web image against the AAO-002 acceptance criteria and starts it:
#   * smaller than MAX_IMAGE_MB (default 60)
#   * runs as UID 10001 and ships no shell
#   * starts, /healthz and /readyz answer 200 on the public port, /metrics is NOT on it
#
# Usage: scripts/image-smoke.sh <image> [worker-image]
set -euo pipefail

image="${1:?usage: image-smoke.sh <web-image> [worker-image]}"
worker_image="${2:-}"
max_mb="${MAX_IMAGE_MB:-60}"
name="aao-smoke-$$"
port="${SMOKE_PORT:-18080}"

cleanup() { docker rm -f "$name" >/dev/null 2>&1 || true; }
trap cleanup EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }

check_static() {
  local img="$1" label="$2" bytes user
  bytes="$(docker image inspect --format '{{.Size}}' "$img")"
  echo "$label: $((bytes / 1024 / 1024)) MiB (limit ${max_mb})"
  (( bytes < max_mb * 1024 * 1024 )) || fail "$label image is ${bytes} bytes, limit ${max_mb} MiB"

  user="$(docker image inspect --format '{{.Config.User}}' "$img")"
  [[ "$user" == "10001:10001" || "$user" == "10001" ]] || fail "$label runs as '${user}', expected 10001"

  # Distroless: neither a shell nor common busybox-style tools may exist.
  for sh in /bin/sh /bin/bash /busybox/sh; do
    if docker run --rm --entrypoint "$sh" "$img" -c true >/dev/null 2>&1; then
      fail "$label contains a shell at $sh"
    fi
  done
}

check_static "$image" web
[[ -z "$worker_image" ]] || check_static "$worker_image" worker

docker run -d --name "$name" -p "127.0.0.1:${port}:8080" "$image" >/dev/null

for _ in $(seq 1 30); do
  if curl -fsS "http://127.0.0.1:${port}/healthz" >/dev/null 2>&1; then break; fi
  sleep 1
done
[[ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:${port}/healthz")" == "200" ]] \
  || { docker logs "$name" >&2; fail "/healthz did not return 200"; }
[[ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:${port}/readyz")" == "200" ]] \
  || { docker logs "$name" >&2; fail "/readyz did not return 200 (no database configured)"; }
[[ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:${port}/metrics")" == "404" ]] \
  || fail "/metrics must not be served on the public port"
[[ "$(docker inspect --format '{{.State.Running}}' "$name")" == "true" ]] || fail "container exited"

uid="$(docker top "$name" -o uid | tail -n +2 | head -n1 | tr -d '[:space:]')"
[[ "$uid" == "10001" ]] || fail "process runs as uid '${uid}', expected 10001"

if [[ -n "$worker_image" ]]; then
  out="$(docker run --rm "$worker_image")"
  [[ "$out" == worker:* ]] || fail "worker stub did not start: '${out}'"
fi
echo "image smoke OK"
