---
name: mem9-setup
description: Use when Mem9 is not initialized for this agent, when a skill reports missing MEM9_TENANT_ID, or when the user asks to install or configure Mem9 memory for a globally installed agent skill.
---

# mem9 Setup for Global Agent Skills

You are an automated Mem9 setup agent. Configure Mem9 so globally installed agent skills can reconnect to the same memory space through environment variables.

## Terminology

Use this distinction consistently:

- `tenantID` is the internal API field name.
- `MEM9_TENANT_ID` is the environment variable name.
- `space ID` is the preferred user-facing term.

Treat the space ID like a secret. Anyone who has it can access that Mem9 space.

## Step 1: Choose or Provide a Mem9 Space

Ask which setup the user wants:

1. Create a new mem9 space
2. Reconnect an existing mem9 space

If the user already has a space ID, verify it before using it.

### Verify an Existing Space

First check API reachability:

```bash
curl -sf --max-time 8 "https://api.mem9.ai/healthz" \
  && echo "API_OK" || echo "API_UNREACHABLE"
```

If the API is reachable, verify the space ID:

```bash
curl -sf --max-time 8 \
  "https://api.mem9.ai/v1alpha1/mem9s/$SPACE_ID/memories?limit=1" \
  && echo "SPACE_OK" || echo "SPACE_INVALID"
```

If `SPACE_INVALID`, tell the user to re-check the space ID or create a new one.

## Step 2: Create a New Mem9 Space If Needed

If the user needs a new space:

```bash
curl -sX POST https://api.mem9.ai/v1alpha1/mem9s
```

Save the returned `id` as the user's space ID and as `MEM9_TENANT_ID`.

Tell the user:

- The new mem9 space is ready.
- This space ID reconnects to the same memory from any machine.
- The space ID is secret and should never be shared.

## Step 3: Configure the Environment

Globally installed skills should use environment variables, not agent-specific config files.

Set the environment variable before launching the agent:

```bash
export MEM9_TENANT_ID=""
```

Tell the user that `MEM9_TENANT_ID` must be present in the local environment used to launch the agent.

Do not modify shell RC files directly.

If the user asks how to persist it, provide guidance only. For example:

```bash
# zsh
echo 'export MEM9_TENANT_ID=""' >> ~/.zshrc

# bash
echo 'export MEM9_TENANT_ID=""' >> ~/.bashrc
```

## Step 4: Verify Setup

After the environment is configured, verify the setup:

- `mem9-recall` should stop reporting missing `MEM9_TENANT_ID`
- `mem9-store` should be able to write a memory
- `using-mem9` direct CRUD commands should work when launched from the same environment

Quick verification flow:

1. Ask the agent to remember a simple fact.
2. Start a new session from the same environment.
3. Ask the agent to recall that fact.

## What to Tell the User

When setup is complete, report:

- the space ID
- that `MEM9_TENANT_ID` must be exported in the local environment
- that the same space ID reconnects to the same memory on any machine
- that they should store the space ID somewhere safe

## Troubleshooting

- If you see `No MEM9_TENANT_ID configured`, export `MEM9_TENANT_ID` before starting the agent.
- If API calls return `404`, verify the space ID.
- If the API is unreachable, check connectivity to `api.mem9.ai`.
