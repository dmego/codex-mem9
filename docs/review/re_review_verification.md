# codex-mem9 二次审查验证报告

> 基于 Codex 修复后的代码进行复核，原始报告共 20 项，其中 5 项被判定为不成立已剔除。

---

## ✅ 已修复的问题

### 🔴 P0-1：`run_watch` 优雅退出 → ✅ 已修复

[lib.rs:19-67](file:///Users/dmego/vibeCoding/codex-mem9/src/lib.rs#L19-L67)

- 新增 `shutdown_signal()` 函数，监听 `SIGTERM` + `Ctrl-C`
- 新增 `wait_for_next_cycle()` 使用 `tokio::select!` 在信号和 sleep 间选择
- `#[cfg(unix)]` / `#[cfg(not(unix))]` 条件编译处理跨平台
- 新增 2 个单元测试验证 shutdown 和 tick 行为

**评价**：实现完整、测试到位。✅

---

### 🔴 P0-2：`state.json` 原子写入 → ✅ 已修复

[state.rs:24-36](file:///Users/dmego/vibeCoding/codex-mem9/src/state.rs#L24-L36)

- 先写入 `.tmp` 临时文件，再 `fs::rename` 原子替换
- 新增单元测试验证 round-trip 且 `.tmp` 文件不残留

**评价**：符合预期。✅

---

### 🔴 P0-3：`sync_once` 错误恢复与速率限制 → ✅ 已修复

[importer.rs:32-57](file:///Users/dmego/vibeCoding/codex-mem9/src/importer.rs#L32-L57)

- HTTP 错误不再 `?` 中断，改为 `eprintln!` 记录并 `continue`
- 每条导入后立即 `state.save()` 保存进度
- 新增 `STORE_DELAY_MS = 100` 请求间隔
- 新增 `httpmock` 集成测试验证 500 错误时继续运行

**评价**：核心改进到位。✅

---

### 🟠 P1-4：`mark_imported` 淘汰策略 → ✅ 已修复

[state.rs:38-43](file:///Users/dmego/vibeCoding/codex-mem9/src/state.rs#L38-L43)

- `BTreeSet` 改为 `IndexSet`，维护插入顺序
- `shift_remove_index(0)` 正确移除最早插入的条目
- 新增淘汰策略测试

**评价**：语义正确。✅

---

### 🟠 P1-5：`brew.rs` 死代码 → ✅ 已修复

- `src/brew.rs` 已删除
- `lib.rs` 中 `pub mod brew;` 已移除

**评价**：清理干净。✅

---

### 🟠 P1-7：`api_key` 回退 `tenant_id` → ✅ 已修复

[config.rs:51-55](file:///Users/dmego/vibeCoding/codex-mem9/src/config.rs#L51-L55)

- `api_key` 从 `Option<String>` 改为 `String`（必填）
- 新增 `.context()` 错误信息引导用户配置
- `Mem9Client::new()` 不再做 fallback

**评价**：消除了安全隐患。✅

---

### 🟠 P1-8：Skills API 版本和认证方式不一致 → ✅ 已修复

| Skill | 修复前 | 修复后 |
|-------|--------|--------|
| `mem9-recall` | `v1alpha1` + URL 路径认证 | `v1alpha2` + `X-API-Key` header |
| `mem9-store` | `v1alpha1` + URL 路径认证 | `v1alpha2` + `X-API-Key` header |
| `mem9-setup` | `v1alpha1` 混用 | `v1alpha2` + `X-API-Key` header |
| `using-mem9` | 已经是 `v1alpha2` | 移除了 `${MEM9_API_KEY:-$MEM9_TENANT_ID}` fallback |

所有 skills 的 preconditions 也更新为同时要求 `MEM9_TENANT_ID` 和 `MEM9_API_KEY`。

**评价**：全面统一。✅

---

### 🟠 P1-13：`build_source` 日期导致重复导入 → ✅ 已修复

[importer.rs:155](file:///Users/dmego/vibeCoding/codex-mem9/src/importer.rs#L155)

```rust
// 修复前
format!("codex-memory:{}:{}", Utc::now().format("%Y%m%d"), ...)
// 修复后
format!("codex-memory:{}", ...)
```

- 日期已移除，`source` 现在是稳定的
- 新增 `build_source_is_stable_for_the_same_path` 测试

**评价**：去重机制不再被日期干扰。✅

---

### 🟡 P1-12：HTTP 客户端缺少超时 → ✅ 已修复

[mem9.rs:30-31](file:///Users/dmego/vibeCoding/codex-mem9/src/mem9.rs#L30-L31)

```rust
.connect_timeout(Duration::from_secs(10))
.timeout(Duration::from_secs(30))
```

**评价**：符合预期。✅

---

### 🔵 P3-15：`Mem9Client::api_key()` 无用 getter → ✅ 已修复

- `api_key` 字段和 `api_key()` 方法已从 `Mem9Client` struct 中移除

**评价**：干净。✅

---

## ❌ 用户判定不成立的问题（已剔除）

| # | 原始问题 | 不成立原因 |
|---|---------|-----------|
| 9 | `edition = "2024"` 需降级 | CI 和本地均能支持，非缺陷 |
| 16 | `Lazy` 应换 `LazyLock` | 技术偏好，不影响正确性 |
| 17 | `println!` 应改结构化日志 | 工程建议，不阻塞功能 |
| 19 | Release workflow 仅 ubuntu | 源码包发布，Homebrew 本地编译 |
| 20 | `.gitignore` 太简单 | 风格建议，未造成实际问题 |

---

## 🔶 仍存在的问题

### 1. `simplify_inline_code` 过度脱敏（原 P2-10）

[redact.rs:79](file:///Users/dmego/vibeCoding/codex-mem9/src/redact.rs#L79)

```rust
if code.starts_with('~') || (code.contains('/') && !code.contains(' ')) {
    return "related path".to_string();
}
```

**未修复**。`content/json`、`v1/api`、`true/false` 等正常内容仍会被误判为路径。

**严重性**：🟡 低。对有价值的记忆内容造成信息损失，但不会导致功能故障。

---

### 2. `collect_tagged_bullets` heading 层级匹配不精确（原 P2-11）

[importer.rs:124](file:///Users/dmego/vibeCoding/codex-mem9/src/importer.rs#L124)

```rust
if line.starts_with("##") {
```

**未修复**。`##`、`###`、`####` 都会匹配，当文档存在嵌套子标题时可能错误归类内容。

**严重性**：🟡 低。当前目标文件（`MEMORY.md`、`memory_summary.md`）结构简单，实际触发概率不高。

---

### 3. `brew.rs` 模板与 `Formula/codex-mem9.rb` 分歧（原 P1-6）→ ✅ 随 `brew.rs` 删除而消解

不再存在分歧，因为 `brew.rs` 已被删除，仅保留 `Formula/codex-mem9.rb` 静态文件。

---

### 4. `Cargo.toml` 中 `chrono` 和 `camino` 依赖已不再使用（新发现）

[Cargo.toml:10-11](file:///Users/dmego/vibeCoding/codex-mem9/Cargo.toml#L10-L11)

```toml
camino = { version = "1.1", features = ["serde1"] }
chrono = { version = "0.4", features = ["serde"] }
```

`build_source` 移除日期后不再使用 `chrono::Utc`，`camino` 在项目中也没有任何引用。这两个依赖可以安全移除以减少编译时间和二进制体积。

**严重性**：🔵 低。不影响正确性，仅增加无用编译。

---

## 📊 修复总结

| 状态 | 数量 |
|------|------|
| ✅ 已修复 | 10 |
| ❌ 剔除（不成立） | 5 |
| 🟡 未修复（低优先级） | 2 |
| 🔵 新发现（低优先级） | 1 |
| ✅ 随删除消解 | 1 |

> 所有 P0 和 P1 问题均已修复或消解。剩余未修复项均为低优先级，不影响核心功能。
