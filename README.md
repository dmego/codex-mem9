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

Set the required environment variables before using the skills or the CLI:

```bash
export MEM9_TENANT_ID="<your-tenant-id>"
export MEM9_API_URL="https://api.mem9.ai"
```

If your mem9 deployment expects an API key for `v1alpha2` endpoints, you can also export:

```bash
export MEM9_API_KEY="<your-api-key>"
```

## Install `codex-mem9` with Homebrew

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

1. Install `codex-mem9` with Homebrew.
2. Export `MEM9_TENANT_ID` and `MEM9_API_URL` in the environment used to launch Codex.
3. If your mem9 deployment expects it, export `MEM9_API_KEY` too.
4. Install the required skill folders from `skills/` into the Codex skills directory.
5. Start the background service with `brew services start codex-mem9`.

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
- `url`: `https://github.com/dmego/codex-mem9/archive/refs/tags/v0.1.0.tar.gz`
