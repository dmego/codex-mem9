# codex-mem9

[![Homebrew Version](https://img.shields.io/homebrew/v/codex-mem9?label=homebrew)](https://formulae.brew.sh/formula/codex-mem9)
[![License](https://img.shields.io/github/license/dmego/codex-mem9)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange)](./Cargo.toml)

[中文文档](./README.zh-CN.md)

`codex-mem9` provides two installable parts for AI agents:

- `skills/`: Mem9 skills for Codex-style agents
- `codex-mem9`: a Homebrew-installed CLI and background service that syncs and watches `~/.codex/memories`, redacts sensitive content, and stores sanitized entries into Mem9

## Repository layout

```text
codex-mem9/
├── skills/
│   ├── mem9-recall/
│   ├── mem9-setup/
│   ├── mem9-store/
│   └── using-mem9/
├── src/
├── Cargo.toml
├── Cargo.lock
├── Formula/
├── README.md
└── README.zh-CN.md
```

## What this repository contains

### `skills/`

The `skills/` directory contains agent-readable skills:

- `mem9-recall`: proactively recall relevant memory before answering
- `mem9-store`: persist durable preferences, facts, and decisions into Mem9
- `mem9-setup`: guide environment setup for Mem9
- `using-mem9`: the main Mem9 entrypoint for routing and CRUD usage

These files are meant to be read and installed by AI agent tooling that supports skill directories.

### `codex-mem9`

The Rust CLI provides two commands:

- `sync`: manually import historical Codex memory data
- `watch`: continuously monitor Codex memory data and sync redacted updates

The service reads from `~/.codex/memories`, skips `raw_memories.md`, applies redaction, deduplicates imported content, and stores sanitized entries into Mem9.

## Configuration

Initialization rule:

- Install the required skill folders before you try to run `mem9-setup`
- `mem9-setup` is a skill, not part of the Homebrew formula
- On first use, run `mem9-setup`
- `mem9-setup` should persist `MEM9_TENANT_ID` into both the default shell environment and `~/.codex/config.toml`

Interactive skills and CLI commands use the current process environment first:

```bash
export MEM9_TENANT_ID="<your-tenant-id>"
export MEM9_API_URL="https://api.mem9.ai"
```

`codex-mem9` uses `X-API-Key` when it writes to the `v1alpha2` Mem9 endpoint. In the default Mem9 setup, it reuses the same value as `MEM9_TENANT_ID`, so a separate API key is not required.

Only set `MEM9_API_KEY` if your deployment intentionally uses a different value:

```bash
export MEM9_API_KEY="<your-api-key>"
```

`codex-mem9` runtime precedence is:

1. process environment
2. `[codex_mem9]` in `~/.codex/config.toml`

For new Homebrew service installs, do not rely on `launchctl setenv`. Keep the persistent service configuration in `~/.codex/config.toml`:

```toml
[codex_mem9]
tenant_id = "<your-tenant-id>"
api_url = "https://api.mem9.ai"
# api_key = "<your-api-key-if-different>"
```

If neither the process environment nor `[codex_mem9]` in `~/.codex/config.toml` provides a tenant, `codex-mem9` exits on startup and writes a clear error into the Homebrew service stderr log.

## Install `codex-mem9` with Homebrew

This installs the CLI and background service only. It does not install the skills.

Homebrew note:

- `brew install codex-mem9` installs the latest published tag from `dmego/tap`.
- The current repository working tree can be ahead of that published tag.
- If the tap formula still references an older published tag, the installed binary and service behavior still follow that older tag until a newer one is published.

```bash
brew tap dmego/tap
brew install codex-mem9
```

Check the installed CLI:

```bash
codex-mem9 --help
```

Run a one-time sync:

```bash
codex-mem9 sync
```

Start the background service:

```bash
brew services start codex-mem9
```

Stop or restart the service:

```bash
brew services stop codex-mem9
brew services restart codex-mem9
brew services list
```

## Install the skills for an AI agent

If your agent supports repository-based skill installation, point it at this repository and install the skill directories under `skills/`.

For agents that read skills directly from disk, copy or link the required skill folders into the agent's global skills directory.

Example target skill folders from this repository:

```text
skills/mem9-recall
skills/mem9-setup
skills/mem9-store
skills/using-mem9
```

After installation, the agent can read the skill definitions directly from the skill folders.

## Install both for Codex

To use the full setup with Codex:

1. Install the required skill folders from `skills/` into the Codex skills directory.
2. Run `mem9-setup`.
3. Install `codex-mem9` with Homebrew.
4. If you use the Homebrew service, confirm that `mem9-setup` wrote `[codex_mem9]` into `~/.codex/config.toml`.
5. Start the background service with `brew services start codex-mem9`.

If you are validating changes from this repository before a matching release tag exists, remember that `brew install codex-mem9` still follows the latest published tag, not the current working tree.

This gives Codex both parts of the integration:

- skill-driven memory recall and memory store behavior
- automatic local memory sync through the Homebrew-managed service

## Formula file

The Homebrew formula in this repository is:

```text
Formula/codex-mem9.rb
```

It uses the published repository path:

- `homepage`: `https://github.com/dmego/codex-mem9`
- `url`: `https://github.com/dmego/codex-mem9/archive/refs/tags/v<version>.tar.gz`
