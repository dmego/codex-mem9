#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FORMULA_PATH="${ROOT_DIR}/Formula/codex-mem9.rb"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-mem9-homebrew-smoke.XXXXXX")"
TAP_NAME="codex/smoke"
TAP_DIR="${WORK_DIR}/homebrew-tap"
TAP_FORMULA_DIR="${TAP_DIR}/Formula"
PORT_FILE="${WORK_DIR}/port"
REQUESTS_FILE="${WORK_DIR}/requests.log"
HOME_DIR="${WORK_DIR}/home"
CONFIG_DIR="${HOME_DIR}/.codex"
MEMORIES_DIR="${CONFIG_DIR}/memories"
SERVER_PID=""

cleanup() {
  if [[ -n "${SERVER_PID}" ]]; then
    kill "${SERVER_PID}" >/dev/null 2>&1 || true
    wait "${SERVER_PID}" >/dev/null 2>&1 || true
  fi
  brew uninstall --force codex-mem9 >/dev/null 2>&1 || true
  brew untap "${TAP_NAME}" >/dev/null 2>&1 || true
  rm -rf "${WORK_DIR}"
}

trap cleanup EXIT

export HOMEBREW_NO_AUTO_UPDATE=1

mkdir -p "${TAP_FORMULA_DIR}"
cp "${FORMULA_PATH}" "${TAP_FORMULA_DIR}/codex-mem9.rb"

brew untap "${TAP_NAME}" >/dev/null 2>&1 || true
brew tap "${TAP_NAME}" "${TAP_DIR}"
brew install --build-from-source "${TAP_NAME}/codex-mem9"

brew test "${TAP_NAME}/codex-mem9"

PORT_FILE="${PORT_FILE}" REQUESTS_FILE="${REQUESTS_FILE}" python3 -u - <<'PY' &
import json
import os
import pathlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

port_file = pathlib.Path(os.environ["PORT_FILE"])
requests_file = pathlib.Path(os.environ["REQUESTS_FILE"])
requests_file.touch()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        return

    def _write(self, status_code, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status_code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/healthz":
            self._write(200, {"status": "ok"})
            return
        self._write(404, {"error": "not-found"})

    def do_POST(self):
        if self.path != "/v1alpha2/mem9s/memories":
            self._write(404, {"error": "not-found"})
            return

        content_length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(content_length).decode("utf-8")
        with requests_file.open("a", encoding="utf-8") as handle:
            handle.write(body + "\n")
        self._write(202, {"status": "accepted"})


server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
port_file.write_text(str(server.server_address[1]), encoding="utf-8")
server.serve_forever()
PY
SERVER_PID=$!

for _ in $(seq 1 50); do
  if [[ -s "${PORT_FILE}" ]]; then
    break
  fi
  sleep 0.1
done

if [[ ! -s "${PORT_FILE}" ]]; then
  echo "failed to start Mem9 mock server" >&2
  exit 1
fi

PORT="$(<"${PORT_FILE}")"
INSTALLED_BIN="$(brew --prefix "${TAP_NAME}/codex-mem9")/bin/codex-mem9"

mkdir -p "${MEMORIES_DIR}"
printf '%s\n' \
  "[codex_mem9]" \
  'tenant_id = "tenant"' \
  "api_url = \"http://127.0.0.1:${PORT}\"" \
  > "${CONFIG_DIR}/config.toml"
printf '%s\n' \
  "### learnings" \
  "- Homebrew smoke memory item" \
  > "${MEMORIES_DIR}/MEMORY.md"

FIRST_OUTPUT="$(
  env \
    -u MEM9_TENANT_ID \
    -u MEM9_API_URL \
    -u MEM9_API_KEY \
    -u CODEX_MEMORIES_DIR \
    -u CODEX_MEM9_STATE_PATH \
    -u CODEX_MEM9_POLL_INTERVAL_SECONDS \
    HOME="${HOME_DIR}" \
    "${INSTALLED_BIN}" sync
)"

if [[ "${FIRST_OUTPUT}" != *"synced total=1 imported=1 skipped=0"* ]]; then
  echo "unexpected first sync output: ${FIRST_OUTPUT}" >&2
  exit 1
fi

SECOND_OUTPUT="$(
  env \
    -u MEM9_TENANT_ID \
    -u MEM9_API_URL \
    -u MEM9_API_KEY \
    -u CODEX_MEMORIES_DIR \
    -u CODEX_MEM9_STATE_PATH \
    -u CODEX_MEM9_POLL_INTERVAL_SECONDS \
    HOME="${HOME_DIR}" \
    "${INSTALLED_BIN}" sync
)"

if [[ "${SECOND_OUTPUT}" != *"synced total=1 imported=0 skipped=1"* ]]; then
  echo "unexpected second sync output: ${SECOND_OUTPUT}" >&2
  exit 1
fi

REQUEST_COUNT="$(wc -l < "${REQUESTS_FILE}" | tr -d ' ')"
if [[ "${REQUEST_COUNT}" != "1" ]]; then
  echo "expected exactly one Mem9 store request, got ${REQUEST_COUNT}" >&2
  exit 1
fi
