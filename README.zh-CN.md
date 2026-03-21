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

初始化规则：

- 先安装所需的 skill 目录，再运行 `mem9-setup`
- `mem9-setup` 是 skill，不属于 Homebrew formula
- 第一次使用先运行 `mem9-setup`
- `mem9-setup` 应把 `MEM9_TENANT_ID` 同时写入启动 Codex 的默认 shell 环境和 `~/.codex/config.toml`

交互式 skills 和 CLI 命令会优先读取当前进程环境变量：

```bash
export MEM9_TENANT_ID="<your-tenant-id>"
export MEM9_API_URL="https://api.mem9.ai"
```

`codex-mem9` 在写入 `v1alpha2` Mem9 接口时会使用 `X-API-Key`。在默认的 Mem9 用法里，它会直接复用 `MEM9_TENANT_ID` 的值，因此通常不需要额外的 API key。

只有在你的部署明确使用不同值时，才需要额外设置 `MEM9_API_KEY`：

```bash
export MEM9_API_KEY="<your-api-key>"
```

`codex-mem9` 的运行时读取顺序是：

1. 进程环境变量
2. `~/.codex/config.toml` 里的 `[codex_mem9]`

对于新的 `brew services` 安装，不要依赖 `launchctl setenv`。后台服务的持久化配置应写到 `~/.codex/config.toml`：

```toml
[codex_mem9]
tenant_id = "<your-tenant-id>"
api_url = "https://api.mem9.ai"
# api_key = "<your-api-key-if-different>"
```

如果进程环境变量和 `~/.codex/config.toml` 中的 `[codex_mem9]` 都没有提供 tenant，`codex-mem9` 会在启动时退出，并把明确的错误信息写到 Homebrew service 的 stderr 日志里。

## 通过 Homebrew 安装 `codex-mem9`

这里只安装 CLI 和后台服务，不会安装 skills。

Homebrew 说明：

- `brew install codex-mem9` 安装的是 `dmego/tap` 里最新已发布的 tag。
- 当前仓库工作树可能已经领先于这个已发布 tag。
- 如果 tap formula 仍然指向较旧的已发布 tag，那么实际安装出来的二进制和服务行为仍然会继续跟随那个旧 tag，直到新的 tag 发布为止。

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

1. 先把 `skills/` 里的所需目录安装到 Codex 的 skills 目录。
2. 运行 `mem9-setup`。
3. 通过 Homebrew 安装 `codex-mem9`。
4. 如果你使用 Homebrew 后台服务，确认 `mem9-setup` 已把 `[codex_mem9]` 写入 `~/.codex/config.toml`。
5. 通过 `brew services start codex-mem9` 启动后台服务。

如果你是在匹配 release tag 之前先验证本仓库里的变更，要记住：`brew install codex-mem9` 仍然跟随最新已发布 tag，而不是当前工作树。

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
- `url`：`https://github.com/dmego/codex-mem9/archive/refs/tags/v<version>.tar.gz`
