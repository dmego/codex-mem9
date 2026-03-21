# codex-mem9

[![Homebrew Version](https://img.shields.io/homebrew/v/codex-mem9?label=homebrew)](https://formulae.brew.sh/formula/codex-mem9)
[![License](https://img.shields.io/github/license/dmego/codex-mem9)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange)](./Cargo.toml)

[English README](./README.md)

`codex-mem9` 提供两部分可安装内容，供 AI agent 使用：

- `skills/`：给 Codex 一类 agent 使用的 Mem9 skills
- `codex-mem9`：通过 Homebrew 安装的 CLI 和后台服务，用于同步和监控 `~/.codex/memories`，脱敏后写入 Mem9

## 目录结构

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

## 仓库内容

### `skills/`

`skills/` 目录包含可被 agent 直接读取的 skill：

- `mem9-recall`：回答前主动回忆相关记忆
- `mem9-store`：把长期偏好、事实和决策写入 Mem9
- `mem9-setup`：指导完成 Mem9 环境配置
- `using-mem9`：统一的 Mem9 使用入口和 CRUD 指引

这些目录用于支持 skill 机制的 agent 直接读取和安装。

### `codex-mem9`

Rust CLI 提供两个命令：

- `sync`：手动导入历史 Codex memory 数据
- `watch`：持续监控 Codex memory 数据并同步脱敏后的更新

服务会读取 `~/.codex/memories`，跳过 `raw_memories.md`，执行脱敏、去重，并把清洗后的内容写入 Mem9。

## 配置

在交互式使用 skills 或 CLI 之前，先设置这些环境变量：

```bash
export MEM9_TENANT_ID="<your-tenant-id>"
export MEM9_API_URL="https://api.mem9.ai"
```

如果你的 mem9 部署在 `v1alpha2` 接口上要求 API key，也可以额外导出：

```bash
export MEM9_API_KEY="<your-api-key>"
```

对于 `brew services`，launchd 不会读取交互式 shell 配置。启动 `codex-mem9` 服务前，使用下面两种方式之一：

1. 把运行配置写入 `~/Library/Application Support/ai.dmego.codex-mem9/config.toml`：

```toml
tenant_id = "<your-tenant-id>"
api_url = "https://api.mem9.ai"
# api_key = "<your-api-key>"
```

2. 或者先把环境变量写入 launchd，再重启服务：

```bash
launchctl setenv MEM9_TENANT_ID "<your-tenant-id>"
launchctl setenv MEM9_API_URL "https://api.mem9.ai"
# launchctl setenv MEM9_API_KEY "<your-api-key>"
brew services restart codex-mem9
```

## 通过 Homebrew 安装 `codex-mem9`

```bash
brew tap dmego/tap
brew install codex-mem9
```

检查已安装的 CLI：

```bash
codex-mem9 --help
```

执行一次手动同步：

```bash
codex-mem9 sync
```

启动后台服务：

```bash
brew services start codex-mem9
```

停止或重启服务：

```bash
brew services stop codex-mem9
brew services restart codex-mem9
brew services list
```

## 为 AI agent 安装 skills

如果你的 agent 支持从仓库安装 skill，可以直接读取本仓库并安装 `skills/` 下的目录。

如果 agent 需要从本地目录读取 skill，就把所需的 skill 目录复制或软链到 agent 的全局 skills 目录中。

本仓库中的 skill 目录如下：

```text
skills/mem9-recall
skills/mem9-setup
skills/mem9-store
skills/using-mem9
```

安装完成后，agent 就可以直接读取这些 skill 目录中的定义。

## 为 Codex 同时安装两部分

如果要给 Codex 完整接入：

1. 通过 Homebrew 安装 `codex-mem9`。
2. 在启动 Codex 的环境中导出 `MEM9_TENANT_ID` 和 `MEM9_API_URL`。
3. 如果你的 mem9 部署要求，也导出 `MEM9_API_KEY`。
4. 把 `skills/` 里的所需目录安装到 Codex 的 skills 目录。
5. 先为 Homebrew 服务配置 launchd 环境或配置文件。
6. 通过 `brew services start codex-mem9` 启动后台服务。

这样 Codex 会同时具备两部分能力：

- 由 skills 驱动的 memory recall 和 memory store 行为
- 由 Homebrew 托管服务提供的本地 memory 自动同步能力

## Formula 文件

本仓库中的 Homebrew Formula 是：

```text
Formula/codex-mem9.rb
```

它使用以下发布仓库路径：

- `homepage`：`https://github.com/dmego/codex-mem9`
- `url`：`https://github.com/dmego/codex-mem9/archive/refs/tags/v0.1.1.tar.gz`
