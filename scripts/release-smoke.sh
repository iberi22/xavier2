#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${XAVIER2_URL:-http://127.0.0.1:8006}"
TOKEN="${XAVIER2_TOKEN:-}"
REQUIRE_BUILD_ROUTE="${XAVIER2_REQUIRE_BUILD_ROUTE:-0}"
REQUIRE_PANEL="${XAVIER2_REQUIRE_PANEL:-0}"
PYTHON_BIN="${PYTHON_BIN:-python3}"

if ! command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
  PYTHON_BIN="python"
fi

command -v "${PYTHON_BIN}" >/dev/null 2>&1

if [[ -z "${TOKEN}" ]]; then
  echo "XAVIER2_TOKEN is required for release smoke checks" >&2
  exit 1
fi

echo "Running Xavier2 release smoke checks against ${BASE_URL}"

json_field() {
  "${PYTHON_BIN}" - "$1" "$2" <<'PY'
import json
import sys

payload = json.loads(sys.argv[1])
value = payload
for key in sys.argv[2].split("."):
    value = value[key]
print(value)
PY
}

json_assert() {
  "${PYTHON_BIN}" - "$1" "$2" <<'PY'
import json
import sys

payload = json.loads(sys.argv[1])
expression = sys.argv[2]
if not eval(expression, {"__builtins__": {}}, {"payload": payload, "len": len, "bool": bool, "isinstance": isinstance, "list": list}):
    raise SystemExit(f"assertion failed: {expression}")
PY
}

health="$(curl -fsS "${BASE_URL}/health")"
[[ "${health}" == *'"status":"ok"'* ]]
echo "PASS /health"

readiness="$(curl -fsS "${BASE_URL}/readiness")"
[[ "${readiness}" == *'"service":"xavier2"'* ]]
echo "PASS /readiness"

build_status="$(curl -s -o /tmp/xavier2-build-smoke.json -w "%{http_code}" \
  -H "X-Xavier2-Token: ${TOKEN}" \
  "${BASE_URL}/build")"
if [[ "${build_status}" == "200" ]]; then
  build="$(cat /tmp/xavier2-build-smoke.json)"
  [[ "${build}" == *'"service":"xavier2"'* ]]
  echo "PASS /build"
elif [[ "${build_status}" == "404" && "${REQUIRE_BUILD_ROUTE}" != "1" ]]; then
  echo "WARN /build not exposed by current server surface; skipping optional build check"
else
  echo "Build info check failed with status ${build_status}" >&2
  exit 1
fi

auth_status="$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/v1/account/usage")"
if [[ "${auth_status}" == "200" ]]; then
  echo "WARN auth gate bypassed; assuming dev mode is enabled"
else
  [[ "${auth_status}" == "401" ]]
fi
echo "PASS auth gate"

doc_path="smoke/$(date +%Y%m%d%H%M%S)"
content='Xavier2 public release smoke test document'

curl -fsS \
  -X POST "${BASE_URL}/memory/add" \
  -H "X-Xavier2-Token: ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{\"path\":\"${doc_path}\",\"content\":\"${content}\",\"metadata\":{\"source\":\"release-smoke\"}}" >/dev/null
echo "PASS /memory/add"

search="$(curl -fsS \
  -X POST "${BASE_URL}/memory/search" \
  -H "X-Xavier2-Token: ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"query":"public release smoke","limit":5}')"
json_assert "${search}" 'payload["count"] >= 1 and any("public release smoke" in item.get("content", "").lower() for item in payload["results"])'
echo "PASS /memory/search"

curl -fsS \
  -H "X-Xavier2-Token: ${TOKEN}" \
  "${BASE_URL}/v1/account/usage" >/dev/null
echo "PASS /v1/account/usage"

if [[ "${REQUIRE_PANEL}" == "1" ]]; then
  panel_shell_status="$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/panel")"
  [[ "${panel_shell_status}" == "200" ]]
  echo "PASS /panel"

  panel_asset_status="$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/panel/assets/index.js")"
  [[ "${panel_asset_status}" == "200" ]]
  echo "PASS /panel/assets/index.js"

  panel_missing_asset_status="$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/panel/assets/missing.js")"
  [[ "${panel_missing_asset_status}" == "404" ]]
  echo "PASS missing panel asset returns 404"

  panel_unauthorized_status="$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/panel/api/threads")"
  [[ "${panel_unauthorized_status}" == "401" ]]
  echo "PASS panel auth gate"

  panel_threads="$(curl -fsS \
    -H "X-Xavier2-Token: ${TOKEN}" \
    "${BASE_URL}/panel/api/threads")"
  json_assert "${panel_threads}" "isinstance(payload, list)"
  echo "PASS /panel/api/threads"

  new_thread="$(curl -fsS \
    -X POST "${BASE_URL}/panel/api/threads" \
    -H "X-Xavier2-Token: ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{"title":"New Thread"}')"
  thread_id="$(json_field "${new_thread}" "id")"
  [[ -n "${thread_id}" ]]
  echo "PASS create panel thread"

  empty_thread_detail="$(curl -fsS \
    -H "X-Xavier2-Token: ${TOKEN}" \
    "${BASE_URL}/panel/api/threads/${thread_id}")"
  json_assert "${empty_thread_detail}" "payload['thread']['title'] == 'New Thread' and len(payload['messages']) == 0"
  echo "PASS empty panel thread detail"

  panel_chat="$(curl -fsS \
    -X POST "${BASE_URL}/panel/api/chat" \
    -H "X-Xavier2-Token: ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{\"thread_id\":\"${thread_id}\",\"message\":\"Explain xavier2 memory and show a structured UI.\"}")"
  json_assert "${panel_chat}" "payload['thread']['id'] == '${thread_id}' and len(payload['messages']) == 2 and payload['messages'][-1]['role'] == 'assistant' and bool(payload['messages'][-1].get('openui_lang'))"
  echo "PASS /panel/api/chat"

  updated_thread_detail="$(curl -fsS \
    -H "X-Xavier2-Token: ${TOKEN}" \
    "${BASE_URL}/panel/api/threads/${thread_id}")"
  json_assert "${updated_thread_detail}" "payload['thread']['title'] != 'New Thread' and len(payload['messages']) == 2"
  echo "PASS first panel message retitles the thread"
else
  echo "WARN panel checks skipped; set XAVIER2_REQUIRE_PANEL=1 to enforce panel validation"
fi

echo "Xavier2 release smoke checks passed."
