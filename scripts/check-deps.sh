#!/usr/bin/env bash
# Dependency-direction check (docs/ARCHITECTURE.md §4, AGENTS.md §3).
#
# Fails when a workspace crate depends on something the crate map does not allow:
#   * a workspace edge that is not in ALLOWED (e.g. domain -> db, anything -> web/worker),
#   * an external I/O crate outside the crates permitted to use it (ONLY_IN),
#   * an I/O-capable crate anywhere in domain's dependency closure (DOMAIN_FORBIDDEN),
#   * a crate that is not in the crate map at all (a tenth crate needs an ADR).
#
# Changing the maps below changes the architecture: it needs owner review (CODEOWNERS).
set -euo pipefail
cd "${1:-$(dirname "$0")/..}"

declare -A ALLOWED=(
  [domain]=""
  [db]="domain"
  [ledger]="domain"
  [affiliate]="domain"
  [app]="domain db ledger affiliate"
  [ui]="domain"
  [web]="app ui domain"
  [worker]="app domain"
  [testkit]="domain db ledger affiliate app"
)

# External crates that only specific workspace crates may depend on directly
# (normal + build dependencies; dev-dependencies are exempt).
declare -A ONLY_IN=(
  [sqlx]="db ledger testkit"        # SQL access; only ledger writes ledger_* tables (reviewed)
  [reqwest]="affiliate testkit"     # outbound HTTP: providers and user-supplied URLs (AGENTS.md §3)
  [axum]="web"
)

# domain must stay pure: none of these may appear anywhere in its dependency closure.
DOMAIN_FORBIDDEN="tokio async-std smol hyper axum reqwest sqlx leptos"

fail=0
err() { echo "ERROR: $*" >&2; fail=1; }

# Capture into variables (not process substitution) so a failing cargo/jq aborts the script
# under `set -e` instead of silently producing an empty list and a false "OK".
meta=$(cargo metadata --format-version 1 --no-deps)
crate_names=$(jq -r '.packages[].name' <<<"$meta" | sort)
mapfile -t crates <<<"$crate_names"
if [[ ${#crates[@]} -eq 0 || -z "${crates[0]}" ]]; then
  echo "ERROR: no workspace crates found; refusing to report success" >&2
  exit 1
fi

for crate in "${crates[@]}"; do
  if [[ ! -v ALLOWED[$crate] ]]; then
    err "crate '$crate' is not in the crate map (docs/ARCHITECTURE.md §4); adding a crate needs an ADR"
    continue
  fi

  # name, path-or-null, kind (normal|dev|build)
  dep_lines=$(jq -r --arg c "$crate" \
    '.packages[] | select(.name == $c) | .dependencies[] | "\(.name) \(.path) \(.kind // "normal")"' <<<"$meta")
  while read -r dep path kind; do
    [[ -z "$dep" ]] && continue
    if [[ "$path" != "null" ]]; then
      # Workspace-internal edge.
      if [[ "$kind" == "dev" ]]; then
        if [[ "$dep" == "testkit" ]]; then
          if [[ "$crate" == "domain" ]]; then err "domain must not dev-depend on testkit (it pulls in I/O crates)"; fi
          continue
        fi
      fi
      if [[ " ${ALLOWED[$crate]} " != *" $dep "* ]]; then
        err "$crate -> $dep ($kind) is not allowed by the crate map"
      fi
    else
      # External crate.
      if [[ " $DOMAIN_FORBIDDEN " == *" $dep "* && "$crate" == "domain" ]]; then
        err "domain depends on '$dep' (I/O crates are forbidden in domain)"
      fi
      if [[ -v ONLY_IN[$dep] && "$kind" != "dev" && " ${ONLY_IN[$dep]} " != *" $crate "* ]]; then
        err "$crate depends on '$dep'; only [${ONLY_IN[$dep]}] may"
      fi
    fi
  done <<<"$dep_lines"
done

# Transitive purity of domain. Needs the resolved graph, so only run once direct checks pass.
# CHECK_DEPS_SKIP_TRANSITIVE=1 is for scripts/tests/selftest.sh only (its fixtures name
# registry crates that cannot be resolved offline); CI never sets it.
if [[ $fail -eq 0 && -z "${CHECK_DEPS_SKIP_TRANSITIVE:-}" && " ${crates[*]} " == *" domain "* ]]; then
  closure=$(cargo tree -p domain -e normal,build --prefix none --locked | awk '{print $1}' | sort -u)
  for bad in $DOMAIN_FORBIDDEN; do
    if grep -qx "$bad" <<<"$closure"; then
      err "domain's dependency closure contains '$bad'"
    fi
  done
fi

if [[ $fail -ne 0 ]]; then
  echo "dependency-direction check FAILED" >&2
  exit 1
fi
echo "dependency-direction check OK (${#crates[@]} crates)"
