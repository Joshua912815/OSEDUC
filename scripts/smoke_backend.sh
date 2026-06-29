#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${OSEDUC_SMOKE_BASE_URL:-http://127.0.0.1:3100}"
BIND_ADDR="${OSEDUC_BIND_ADDR:-127.0.0.1:3100}"
DATABASE_URL="${OSEDUC_DATABASE_URL:-postgres://oseduc:oseduc_dev_password@127.0.0.1:5432/oseduc_ci_smoke}"
ADMIN_TOKEN="${OSEDUC_ADMIN_TOKEN:-ci-admin-token}"
LOG_FILE="${OSEDUC_SMOKE_LOG:-/tmp/oseduc-api-smoke.log}"

cleanup() {
  if [[ -n "${API_PID:-}" ]]; then
    kill "${API_PID}" >/dev/null 2>&1 || true
    wait "${API_PID}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

wait_for_postgres() {
  cargo run -q -p oseduc-store --bin prepare_smoke_database
  cargo run -q -p oseduc-store --bin wait_for_postgres
}

wait_for_api() {
  local attempts=60
  for _ in $(seq 1 "${attempts}"); do
    if curl -fsS "${BASE_URL}/healthz" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  echo "OSeduc API did not become healthy. Recent log:" >&2
  tail -n 80 "${LOG_FILE}" >&2 || true
  return 1
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "${haystack}" != *"${needle}"* ]]; then
    echo "Expected response to contain: ${needle}" >&2
    echo "Response was:" >&2
    echo "${haystack}" >&2
    return 1
  fi
}

export OSEDUC_DATABASE_URL="${DATABASE_URL}"
export OSEDUC_AUTO_MIGRATE=true
export OSEDUC_ENABLE_ADMIN_SEED=true
export OSEDUC_ADMIN_TOKEN="${ADMIN_TOKEN}"
export OSEDUC_BIND_ADDR="${BIND_ADDR}"
export OSEDUC_LLM_PROVIDER=mock

wait_for_postgres

cargo run -p oseduc-api >"${LOG_FILE}" 2>&1 &
API_PID=$!
wait_for_api

seed_response="$(curl -fsS -X POST \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  "${BASE_URL}/v1/admin/knowledge/seed")"
assert_contains "${seed_response}" '"nodes":8'
assert_contains "${seed_response}" '"retrieval_chunks":8'

public_config="$(curl -fsS "${BASE_URL}/v1/config/public")"
assert_contains "${public_config}" '"knowledge_store":"postgres"'
if [[ "${public_config}" == *"oseduc_dev_password"* || "${public_config}" == *"${ADMIN_TOKEN}"* ]]; then
  echo "Public config leaked a database password or admin token" >&2
  exit 1
fi

nodes_response="$(curl -fsS "${BASE_URL}/v1/knowledge/nodes")"
assert_contains "${nodes_response}" "ch1-app-execution-environment"
assert_contains "${nodes_response}" "ch8-concurrency"

progress_response="$(curl -fsS -X PUT \
  -H "content-type: application/json" \
  -d '{"status":"mastered","mastery_score":95,"notes":"ci smoke completed chapter 1"}' \
  "${BASE_URL}/v1/students/ci-smoke/progress/ch1-app-execution-environment")"
assert_contains "${progress_response}" '"status":"mastered"'
assert_contains "${progress_response}" '"mastery_score":95'

learning_path="$(curl -fsS "${BASE_URL}/v1/students/ci-smoke/learning-path?limit=3")"
assert_contains "${learning_path}" "ch2-batch-system"
assert_contains "${learning_path}" '"completed_nodes":1'

tutor_response="$(curl -fsS -X POST \
  -H "content-type: application/json" \
  -d '{"message":"Explain address spaces","knowledge_node_ids":["ch4-address-space"]}' \
  "${BASE_URL}/v1/tutor/chat")"
assert_contains "${tutor_response}" '"label":"rCore v3 ch4"'
assert_contains "${tutor_response}" '"source_grounded_context"'

echo "OSeduc backend smoke test passed"
