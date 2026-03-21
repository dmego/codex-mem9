---
name: mem9-setup
description: Use when Mem9 is not initialized for this agent, when a skill reports missing MEM9_TENANT_ID, or when the user asks to install or configure Mem9 memory for a globally installed agent skill.
---

# Mem9 Setup for Codex

You are an automated installer. Execute the setup steps yourself whenever possible instead of asking the user to edit files manually.

Do not print or echo a full tenant ID in normal output. Only show a masked value such as `abcd...wxyz`.

Use these terms consistently:

- `tenantID`: Mem9 API field name
- `MEM9_TENANT_ID`: Codex skill environment variable
- `space ID`: user-facing term
- `X-API-Key`: the header used by `codex-mem9` for `v1alpha2`; by default it reuses the same value as `MEM9_TENANT_ID` unless the user intentionally overrides `MEM9_API_KEY`

## Setup Goal

After this skill finishes:

1. `MEM9_TENANT_ID` is available in `~/.codex/config.toml` under `[codex_mem9]`.
2. `MEM9_TENANT_ID` is persisted in a shell startup file so future interactive Codex sessions can read it.
3. The current session gets `MEM9_TENANT_ID` immediately.
4. Verification covers the same `v1alpha2` write path used by the `codex-mem9` service.
5. If the user later runs `brew services`, the service can read the tenant from `~/.codex/config.toml`.

## Step 1: Discover Existing Configuration Without Leaking Secrets

Run discovery first. Keep the raw result in a temporary file and print only masked output.

```bash
DISCOVERY_FILE="$(mktemp "${TMPDIR:-/tmp}/codex-mem9-setup.XXXXXX.json")"

DISCOVERY_FILE="$DISCOVERY_FILE" python3 - <<'PY'
import json, os, pathlib, re

def read_text(path):
    p = pathlib.Path(path).expanduser()
    return p.read_text() if p.exists() else ""

def extract_export(path, key):
    text = read_text(path)
    pattern = re.compile(rf'^\s*export\s+{re.escape(key)}\s*=\s*"?(.*?)"?\s*$', re.M)
    match = pattern.search(text)
    return match.group(1).strip() if match and match.group(1).strip() else ""

def extract_codex_block(path):
    text = read_text(path)
    match = re.search(r'(?ms)^\[codex_mem9\]\s*(.*?)(?=^\[|\Z)', text)
    body = match.group(1) if match else ""

    def get_value(key):
        found = re.search(rf'^\s*{re.escape(key)}\s*=\s*"(.*?)"\s*$', body, re.M)
        return found.group(1).strip() if found and found.group(1).strip() else ""

    return {
        "path": str(pathlib.Path(path).expanduser()),
        "tenant_id": get_value("tenant_id"),
        "api_url": get_value("api_url"),
        "api_key": get_value("api_key"),
    }

def mask(value):
    value = (value or "").strip()
    if not value:
        return ""
    if len(value) <= 8:
        return "*" * len(value)
    return f"{value[:4]}...{value[-4:]}"

result = {
    "env": {
        "tenant_id": os.environ.get("MEM9_TENANT_ID", "").strip(),
        "api_url": os.environ.get("MEM9_API_URL", "").strip(),
        "api_key": os.environ.get("MEM9_API_KEY", "").strip(),
    },
    "codex_config": extract_codex_block("~/.codex/config.toml"),
    "zshrc": {
        "path": str(pathlib.Path("~/.zshrc").expanduser()),
        "tenant_id": extract_export("~/.zshrc", "MEM9_TENANT_ID"),
        "api_url": extract_export("~/.zshrc", "MEM9_API_URL"),
        "api_key": extract_export("~/.zshrc", "MEM9_API_KEY"),
    },
    "bashrc": {
        "path": str(pathlib.Path("~/.bashrc").expanduser()),
        "tenant_id": extract_export("~/.bashrc", "MEM9_TENANT_ID"),
        "api_url": extract_export("~/.bashrc", "MEM9_API_URL"),
        "api_key": extract_export("~/.bashrc", "MEM9_API_KEY"),
    },
}

masked = {
    name: {
        "path": data.get("path", ""),
        "tenant_id": mask(data.get("tenant_id", "")),
        "api_url": data.get("api_url", ""),
        "api_key": mask(data.get("api_key", "")),
    }
    for name, data in result.items()
}

path = pathlib.Path(os.environ["DISCOVERY_FILE"])
path.write_text(json.dumps(result))
print(json.dumps({"discovery_file": str(path), "sources": masked}, indent=2))
PY
```

Candidate priority:

1. current environment
2. `~/.codex/config.toml` `[codex_mem9]`
3. `~/.zshrc`
4. `~/.bashrc`

## Step 2: Resolve the Candidate and Verify the Real Service Auth Path

Load the highest-priority candidate without printing secret values:

```bash
eval "$(
  DISCOVERY_FILE="$DISCOVERY_FILE" python3 - <<'PY'
import json, os, shlex

with open(os.environ["DISCOVERY_FILE"], "r", encoding="utf-8") as handle:
    data = json.load(handle)

ordered_sources = [
    data["env"],
    data["codex_config"],
    data["zshrc"],
    data["bashrc"],
]

chosen = {"tenant_id": "", "api_url": "", "api_key": ""}
setup_mode = "provisioned"
for source in ordered_sources:
    tenant = str(source.get("tenant_id", "")).strip()
    if tenant:
        chosen = {
            "tenant_id": tenant,
            "api_url": str(source.get("api_url", "")).strip() or os.environ.get("MEM9_API_URL", "").strip() or "https://api.mem9.ai",
            "api_key": str(source.get("api_key", "")).strip() or os.environ.get("MEM9_API_KEY", "").strip() or tenant,
        }
        setup_mode = "existing"
        break

print(f'export RESOLVED_TENANT_ID={shlex.quote(chosen["tenant_id"])}')
print(f'export RESOLVED_API_URL={shlex.quote(chosen["api_url"] or "https://api.mem9.ai")}')
print(f'export RESOLVED_API_KEY={shlex.quote(chosen["api_key"])}')
print(f'export SETUP_MODE={shlex.quote(setup_mode)}')
PY
)"
```

Check API reachability:

```bash
curl -sf --max-time 8 "${RESOLVED_API_URL:-https://api.mem9.ai}/healthz" >/dev/null \
  && echo "API_OK" || echo "API_UNREACHABLE"
```

If `RESOLVED_TENANT_ID` is present, run two checks:

1. validate the tenant explicitly through the tenant-scoped `v1alpha1` path
2. validate the auth mode used by the `codex-mem9` service through the `v1alpha2` path

Both checks must pass before you reuse the candidate.

```bash
if [ -n "${RESOLVED_TENANT_ID:-}" ]; then
  curl -sf --max-time 8 \
    "${RESOLVED_API_URL:-https://api.mem9.ai}/v1alpha1/mem9s/${RESOLVED_TENANT_ID}/memories?limit=1" >/dev/null \
    && echo "TENANT_OK" || echo "TENANT_INVALID"

  curl -sf --max-time 8 \
    -H "X-API-Key: ${RESOLVED_API_KEY}" \
    "${RESOLVED_API_URL:-https://api.mem9.ai}/v1alpha2/mem9s/memories?limit=1" >/dev/null \
    && echo "SERVICE_AUTH_OK" || echo "SERVICE_AUTH_INVALID"
fi
```

If the candidate is missing or invalid:

- If the user supplied an existing space ID, assign it to `RESOLVED_TENANT_ID`, set `RESOLVED_API_KEY` to `${MEM9_API_KEY:-$RESOLVED_TENANT_ID}`, and rerun both checks above.
- Otherwise provision a new tenant:

```bash
PROVISION_JSON="$(curl -sS -X POST "${RESOLVED_API_URL:-https://api.mem9.ai}/v1alpha1/mem9s")"

eval "$(
  PROVISION_JSON="$PROVISION_JSON" python3 - <<'PY'
import json, os, shlex, sys

payload = json.loads(os.environ["PROVISION_JSON"])
tenant_id = str(payload.get("id", "")).strip()
if not tenant_id:
    raise SystemExit("missing id in Mem9 provision response")

api_key = os.environ.get("MEM9_API_KEY", "").strip() or tenant_id
print(f'export RESOLVED_TENANT_ID={shlex.quote(tenant_id)}')
print(f'export RESOLVED_API_KEY={shlex.quote(api_key)}')
print("export SETUP_MODE=provisioned")
PY
)"
```

At the end of this step you must have:

- `RESOLVED_TENANT_ID`
- `RESOLVED_API_URL`
- `RESOLVED_API_KEY`

## Step 3: Persist to `~/.codex/config.toml`

Write the resolved tenant into Codex persistent config. Preserve unrelated sections.

```bash
RESOLVED_TENANT_ID="$RESOLVED_TENANT_ID" \
RESOLVED_API_URL="${RESOLVED_API_URL:-https://api.mem9.ai}" \
RESOLVED_API_KEY="${RESOLVED_API_KEY:-}" \
python3 - <<'PY'
import os, pathlib, re

tenant = os.environ["RESOLVED_TENANT_ID"].strip()
api_url = os.environ["RESOLVED_API_URL"].strip() or "https://api.mem9.ai"
api_key = os.environ.get("RESOLVED_API_KEY", "").strip()

path = pathlib.Path("~/.codex/config.toml").expanduser()
path.parent.mkdir(parents=True, exist_ok=True)
original = path.read_text() if path.exists() else ""

lines = [
    "[codex_mem9]",
    f'tenant_id = "{tenant}"',
    f'api_url = "{api_url}"',
]
if api_key and api_key != tenant:
    lines.append(f'api_key = "{api_key}"')
new_section = "\n".join(lines) + "\n"

pattern = re.compile(r'(?ms)^\[codex_mem9\]\s*.*?(?=^\[|\Z)')
if pattern.search(original):
    updated = pattern.sub(new_section + "\n", original, count=1)
else:
    updated = original
    if updated and not updated.endswith("\n"):
        updated += "\n"
    if updated:
        updated += "\n"
    updated += new_section

path.write_text(updated)
print(f"UPDATED:{path}")
PY
```

## Step 4: Persist Shell Variables for Future Interactive Sessions

Persist `MEM9_TENANT_ID` so future interactive Codex sessions can use `mem9-recall`, `mem9-store`, and `using-mem9` immediately.

```bash
RESOLVED_TENANT_ID="$RESOLVED_TENANT_ID" \
RESOLVED_API_URL="${RESOLVED_API_URL:-https://api.mem9.ai}" \
RESOLVED_API_KEY="${RESOLVED_API_KEY:-}" \
python3 - <<'PY'
import os, pathlib, re

tenant = os.environ["RESOLVED_TENANT_ID"].strip()
api_url = os.environ["RESOLVED_API_URL"].strip() or "https://api.mem9.ai"
api_key = os.environ.get("RESOLVED_API_KEY", "").strip()
shell = os.environ.get("SHELL", "")

targets = []
if shell.endswith("zsh") or pathlib.Path("~/.zshrc").expanduser().exists():
    targets.append(pathlib.Path("~/.zshrc").expanduser())
if shell.endswith("bash") or pathlib.Path("~/.bashrc").expanduser().exists():
    targets.append(pathlib.Path("~/.bashrc").expanduser())
if not targets:
    targets = [pathlib.Path("~/.zshrc").expanduser()]

def upsert_export(path, key, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    text = path.read_text() if path.exists() else ""
    line = f'export {key}="{value}"'
    pattern = re.compile(rf'^\s*export\s+{re.escape(key)}\s*=.*$', re.M)
    if pattern.search(text):
        text = pattern.sub(line, text)
    else:
        if text and not text.endswith("\n"):
            text += "\n"
        text += line + "\n"
    path.write_text(text)

for path in targets:
    upsert_export(path, "MEM9_TENANT_ID", tenant)
    upsert_export(path, "MEM9_API_URL", api_url)
    if api_key and api_key != tenant:
        upsert_export(path, "MEM9_API_KEY", api_key)
    print(f"UPDATED:{path}")
PY
```

## Step 5: Export the Current Session

Apply the resolved configuration immediately:

```bash
export MEM9_TENANT_ID="$RESOLVED_TENANT_ID"
export MEM9_API_URL="${RESOLVED_API_URL:-https://api.mem9.ai}"
if [ -n "${RESOLVED_API_KEY:-}" ] && [ "${RESOLVED_API_KEY}" != "${RESOLVED_TENANT_ID}" ]; then
  export MEM9_API_KEY="$RESOLVED_API_KEY"
fi
```

## Step 6: Verify End-to-End with the Real `v1alpha2` Write Path

Do not stop at a `v1alpha1` tenant check. `codex-mem9` writes through `v1alpha2` with `X-API-Key`, so setup must verify that write path directly too.

This step validates service write access only. Tenant validity must already have been confirmed in Step 2 through the explicit `v1alpha1` tenant-scoped check.

This verification intentionally stores one tiny memory tagged `setup-check`.

```bash
VERIFY_PAYLOAD='{"content":"codex-mem9 setup verification","tags":["setup-check","codex-mem9"],"source":"codex-mem9:setup-check"}'

HTTP_CODE="$(
  curl -sS -o /tmp/codex-mem9-setup-verify.json -w "%{http_code}" \
    --max-time 8 \
    -X POST \
    -H "Content-Type: application/json" \
    -H "X-API-Key: ${RESOLVED_API_KEY:-${MEM9_API_KEY:-$RESOLVED_TENANT_ID}}" \
    -d "$VERIFY_PAYLOAD" \
    "${RESOLVED_API_URL:-https://api.mem9.ai}/v1alpha2/mem9s/memories"
)"

if [ "$HTTP_CODE" = "202" ] || [ "$HTTP_CODE" = "200" ]; then
  echo "MEM9_SETUP_OK"
else
  echo "MEM9_SETUP_FAILED"
  cat /tmp/codex-mem9-setup-verify.json
fi
```

After verification, remove the temporary discovery file:

```bash
rm -f "$DISCOVERY_FILE"
```

## Important Service Note

`brew services` does not read `~/.zshrc` or `~/.bashrc` automatically. For the Homebrew service, `~/.codex/config.toml` is the durable configuration source. If `[codex_mem9].tenant_id` is missing there, `codex-mem9` will log a clear startup error and exit.

## Completion Report Format

Report:

- setup mode used: `existing` or `provisioned`
- files updated: only file paths, not secret values
- masked tenant ID only
- whether the `v1alpha2` write verification passed
- remind the user that `brew services` depends on `~/.codex/config.toml`

Do not include the full `MEM9_TENANT_ID` in normal output.
