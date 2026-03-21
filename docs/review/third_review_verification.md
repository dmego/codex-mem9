# codex-mem9 第三次审查验证报告

> 验证上次遗留的 3 个问题 + 检查是否引入回退

---

## ✅ 遗留问题全部修复

### 1. `simplify_inline_code` 过度脱敏 → ✅ 已修复

[redact.rs:25-27, 82-84](file:///Users/dmego/vibeCoding/codex-mem9/src/redact.rs#L25-L27)

- 新增 `EXPLICIT_PATH` 正则：`^(~|./|../|/)[^\s]*$`
- 旧逻辑 `code.contains('/') && !code.contains(' ')` 已替换
- `content/json`、`v1/api` 等不再被误判为路径
- 新增 2 个测试覆盖正向和反向场景

---

### 2. `collect_tagged_bullets` heading 层级匹配 → ✅ 已修复

[importer.rs:122-151](file:///Users/dmego/vibeCoding/codex-mem9/src/importer.rs#L122-L151)

- 计算 `heading_level`（`#` 字符数），仅同层级 heading 才更新 `current_heading`
- 遇到不同层级子 heading 时执行 `current_heading.clear()` 停止收集
- 新增 `ignores_nested_headings_when_collecting_bullets` 测试

---

### 3. `chrono` / `camino` 残留依赖 → ✅ 已修复

[Cargo.toml](file:///Users/dmego/vibeCoding/codex-mem9/Cargo.toml)

- `chrono` 和 `camino` 已从 `[dependencies]` 中移除

---

## ⚠️ 发现一处修复回退

### `api_key` 回退到 `Option<String>` + fallback（原 P1-7 回退）

上次审查确认 `api_key` 已改为必填 `String`，但当前代码已回退：

**`config.rs:13`**：`api_key` 类型为 `Option<String>`（无 `.context()` 强制要求）
**`importer.rs:27-30`**：

```rust
let api_key = config
    .api_key
    .clone()
    .unwrap_or_else(|| config.tenant_id.clone());
```

这重新引入了将 `tenant_id` 作为 `X-API-Key` 发送的行为，与 skills 中"必须配置 `MEM9_API_KEY`"的要求不一致。

> [!WARNING]
> 如果这是有意为之（向后兼容旧用户），建议在 README 中明确记录此行为。如果是无意回退，建议恢复 `api_key` 为必填。

---

## 📊 总体状态

| 状态 | 数量 |
|------|------|
| ✅ 全部遗留问题已修复 | 3/3 |
| ⚠️ 修复回退 | 1 |
