#!/usr/bin/env bash
# One command from a bootstrapped VPS to a running stack (AAO-092):
#
#   cp infra/.env.example infra/.env && $EDITOR infra/.env     # once
#   infra/up.sh
#
# Creates missing secrets, validates the compose file, starts every service and waits until all
# healthchecks pass. Safe to re-run (it is also the "apply a new image" step until AAO-094's
# deploy workflow takes over).
set -euo pipefail

infra="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
compose=(docker compose -f "$infra/compose.prod.yml" --env-file "$infra/.env")

[[ -f "$infra/.env" ]] || { echo "missing $infra/.env: copy infra/.env.example and fill it in" >&2; exit 1; }

"$infra/bootstrap/gen-secrets.sh"
"${compose[@]}" config --quiet
"${compose[@]}" pull --quiet
"${compose[@]}" up --detach --wait --wait-timeout 180
"${compose[@]}" ps
echo "stack is up. Grafana: ssh -L 3000:127.0.0.1:3000 <vps>, then http://localhost:3000 (user admin)"
