#!/usr/bin/env bash
# Proves the architecture guard-rails actually fail when they should.
# Each case mutates a scratch copy of the workspace and expects a specific check to reject it.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

passed=0
fresh_copy() {
  rm -rf "$tmp/ws"; mkdir "$tmp/ws"
  (cd "$root" && tar --exclude=./target --exclude=./.git -cf - .) | tar -xf - -C "$tmp/ws"
}
expect_fail() { # description, expected-message-regex, command...
  local desc=$1 pattern=$2; shift 2
  if "$@" >"$tmp/out" 2>&1; then
    echo "SELFTEST FAIL: expected rejection but check passed: $desc" >&2; cat "$tmp/out" >&2; exit 1
  fi
  # The check must fail for the intended reason, not because the fixture is malformed.
  if ! grep -Eq "$pattern" "$tmp/out"; then
    echo "SELFTEST FAIL: '$desc' failed, but not with /$pattern/:" >&2; cat "$tmp/out" >&2; exit 1
  fi
  echo "ok (rejected): $desc"; passed=$((passed + 1))
}

# add_dep <crate> <section> <toml line>: append a line inside [section], creating it if absent.
add_dep() {
  local file="$tmp/ws/crates/$1/Cargo.toml" section=$2 line=$3
  if grep -q "^\[$section\]" "$file"; then
    sed -i "/^\[$section\]/a $line" "$file"
  else
    printf '\n[%s]\n%s\n' "$section" "$line" >>"$file"
  fi
}
expect_pass() {
  local desc=$1; shift
  if ! "$@" >"$tmp/out" 2>&1; then
    echo "SELFTEST FAIL: expected acceptance but check failed: $desc" >&2; cat "$tmp/out" >&2; exit 1
  fi
  echo "ok (accepted): $desc"; passed=$((passed + 1))
}

deps() { CHECK_DEPS_SKIP_TRANSITIVE=1 "$root/scripts/check-deps.sh" "$tmp/ws"; }

# --- dependency direction -------------------------------------------------------------
fresh_copy
expect_pass "pristine workspace passes check-deps" deps

fresh_copy; add_dep domain dependencies 'db = { workspace = true }'
expect_fail "domain -> db" 'domain -> db .* not allowed' deps

fresh_copy; add_dep domain dependencies 'sqlx = "0.8"'
expect_fail "domain -> sqlx" "domain depends on 'sqlx'" deps

fresh_copy; add_dep app dependencies 'reqwest = "0.12"'
expect_fail "app -> reqwest (only affiliate may make outbound calls)" "app depends on 'reqwest'" deps

fresh_copy; add_dep db dependencies 'sqlx = "0.8"'
expect_pass "db -> sqlx is allowed" deps

fresh_copy; add_dep app dependencies 'web = { path = "../web" }'
expect_fail "app -> web (nothing may depend on web)" 'app -> web .* not allowed' deps

fresh_copy; add_dep worker dependencies 'ledger = { workspace = true }'
expect_fail "worker -> ledger (worker only talks to app)" 'worker -> ledger .* not allowed' deps

fresh_copy; add_dep domain dev-dependencies 'testkit = { workspace = true }'
expect_fail "domain dev-depends on testkit" 'domain must not dev-depend on testkit' deps

fresh_copy; add_dep app dev-dependencies 'testkit = { workspace = true }'
expect_pass "app may dev-depend on testkit" deps

fresh_copy
mkdir -p "$tmp/ws/crates/extra/src"
sed 's/^name = "db"/name = "extra"/' "$root/crates/db/Cargo.toml" >"$tmp/ws/crates/extra/Cargo.toml"
echo '' >"$tmp/ws/crates/extra/src/lib.rs"
expect_fail "tenth crate without an ADR" "crate 'extra' is not in the crate map" deps

fresh_copy; echo 'this is not toml' >"$tmp/ws/crates/db/Cargo.toml"
expect_fail "unreadable manifest must fail closed, not report OK" 'failed|error' deps

# --- SQL built with format! ------------------------------------------------------------
sqlcheck() { "$root/scripts/check-no-sql-format.sh" "$tmp/sql"; }
write_rs() { rm -rf "$tmp/sql"; mkdir -p "$tmp/sql"; printf '%s\n' "$1" >"$tmp/sql/x.rs"; }

write_rs 'fn f(id: i64) -> String { format!("SELECT * FROM users WHERE id = {}", id) }'
expect_fail "format! with SELECT" "x.rs:1: SQL built" sqlcheck
write_rs 'fn f(t: &str) { let q = format!(r#"DELETE FROM {t} WHERE 1=1"#); }'
expect_fail "raw-string format! with DELETE" "x.rs:1: SQL built" sqlcheck
write_rs 'fn f(id: i64) { sqlx::query(&format!("UPDATE users SET name = {id}")); }'
expect_fail "sqlx::query(&format!(UPDATE ...))" "x.rs:1: SQL built" sqlcheck
write_rs 'fn f(id: i64) {
    let q = format!(
        "INSERT INTO t (a)
         VALUES ({id})"
    );
}'
expect_fail "multi-line format! with INSERT" "x.rs:2: SQL built" sqlcheck
write_rs 'fn f(id: i64) -> String { format!("user {} was updated", id) }'
expect_pass "format! with ordinary text is fine" sqlcheck
write_rs 'fn f() { let _ = sqlx::query!("SELECT 1 AS one"); }'
expect_pass "static query! macro is fine" sqlcheck

# --- production compose policy (needs docker compose + jq; skipped where they are absent) --------
if docker compose version >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
  infra_check() { "$root/scripts/check-infra.sh" "$tmp/ws/infra/compose.prod.yml"; }
  compose_ws() { echo "$tmp/ws/infra/compose.prod.yml"; }

  fresh_copy
  expect_pass "pristine compose.prod.yml passes check-infra" infra_check

  fresh_copy; sed -i '0,/^  postgres:$/s//  postgres:\n    ports: ["5432:5432"]/' "$(compose_ws)"
  expect_fail "publishing postgres" 'postgres publishes 5432' infra_check

  fresh_copy; sed -i 's/"127.0.0.1:3000:3000"/"3000:3000"/' "$(compose_ws)"
  expect_fail "grafana on a public port" 'grafana publishes 3000' infra_check

  fresh_copy; sed -i 's/^  read_only: true$/  read_only: false/' "$(compose_ws)"
  expect_fail "writable root filesystem" 'read_only must be true' infra_check

  fresh_copy; sed -i 's/^  cap_drop: \[ALL\]$/  cap_drop: [NET_RAW]/' "$(compose_ws)"
  expect_fail "capabilities not dropped" 'cap_drop must contain ALL' infra_check

  fresh_copy; sed -i 's#image: caddy:2.11.4#image: caddy:latest#' "$(compose_ws)"
  expect_fail "latest tag" 'must not use :latest' infra_check

  fresh_copy; sed -i 's#^      APP_ENV: production$#      APP_ENV: production\n      API_TOKEN: hunter2#' "$(compose_ws)"
  expect_fail "secret in environment" 'web sets API_TOKEN' infra_check

  fresh_copy; sed -i 's#^      TRUST_FORWARDED_FOR: "true"$#      TRUST_FORWARDED_FOR: "false"#' "$(compose_ws)"
  expect_fail "web not trusting the proxy header" 'TRUST_FORWARDED_FOR=false' infra_check

  fresh_copy; sed -i 's#^  app: { internal: true }$#  app: {}#' "$(compose_ws)"
  expect_fail "app network not internal" 'must be internal' infra_check
else
  echo "skipped: compose policy checks (docker compose or jq not available)"
fi

echo "selftest: $passed checks behaved as expected"
