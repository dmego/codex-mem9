# Homebrew 发布指南

本文档说明如何发布 `codex-mem9`，并把它同步到 `homebrew-tap`。

## 1. 仓库分工

发布链路包含两个仓库：

- 源码仓库：`https://github.com/dmego/codex-mem9`
- tap 仓库：`https://github.com/dmego/homebrew-tap`

职责划分如下：

- 源码仓库负责代码、tag、GitHub Release 和源码包产物
- tap 仓库负责 `Formula/codex-mem9.rb`，让 Homebrew 知道去哪里下载和安装

Homebrew 实际读取的是 tap 仓库里的 Formula，不直接读取源码仓库中的 Formula 示例文件。

## 2. 先在源码仓库完成发布

在源码仓库中先执行本地校验：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

确认这些文件已经更新到目标版本：

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `README.zh-CN.md`

例如目标版本：

```text
0.1.1
```

对应 tag：

```text
v0.1.1
```

提交并推送源码仓库：

```bash
git add .
git commit -m "release: prepare v0.1.1"
git push origin main
```

创建并推送 tag：

```bash
git tag v0.1.1
git push origin v0.1.1
```

## 3. 等待源码仓库生成 GitHub Release

源码仓库中的 GitHub Actions 会自动执行：

1. 格式检查、clippy、测试
2. release 构建
3. 生成源码包 `codex-mem9-v0.1.1.tar.gz`
4. 生成校验文件 `codex-mem9-v0.1.1.tar.gz.sha256`
5. 创建 GitHub Release 并上传这两个文件

等待 workflow 成功完成。

## 4. 获取 release 产物和 SHA-256

新的 release 产物地址类似：

```text
https://github.com/dmego/codex-mem9/releases/download/v0.1.1/codex-mem9-v0.1.1.tar.gz
```

校验文件地址类似：

```text
https://github.com/dmego/codex-mem9/releases/download/v0.1.1/codex-mem9-v0.1.1.tar.gz.sha256
```

下载并读取 SHA-256：

```bash
curl -L -o codex-mem9-v0.1.1.tar.gz.sha256 \
  https://github.com/dmego/codex-mem9/releases/download/v0.1.1/codex-mem9-v0.1.1.tar.gz.sha256

cat codex-mem9-v0.1.1.tar.gz.sha256
```

内容类似：

```text
<sha256-value>  codex-mem9-v0.1.1.tar.gz
```

只取前面的 hash 值。

## 5. 再更新 tap 仓库

打开 tap 仓库并修改：

```text
Formula/codex-mem9.rb
```

更新以下字段：

- `url`
- `sha256`
- `version`

示例：

```ruby
class CodexMem9 < Formula
  desc "Sync and watch Codex memories into Mem9 with redaction"
  homepage "https://github.com/dmego/codex-mem9"
  url "https://github.com/dmego/codex-mem9/releases/download/v0.1.1/codex-mem9-v0.1.1.tar.gz"
  sha256 "<sha256-value>"
  version "0.1.1"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  service do
    run [opt_bin/"codex-mem9", "watch"]
    keep_alive true
    log_path var/"log/codex-mem9.log"
    error_log_path var/"log/codex-mem9.err.log"
  end

  test do
    assert_match "sync", shell_output("#{bin}/codex-mem9 --help")
  end
end
```

提交并推送 tap 仓库：

```bash
git add Formula/codex-mem9.rb
git commit -m "codex-mem9 0.1.1"
git push origin main
```

## 6. 验证安装

更新 tap 并安装：

```bash
brew tap dmego/tap
brew update
brew install codex-mem9
```

验证 CLI：

```bash
codex-mem9 --help
```

验证服务：

```bash
brew services start codex-mem9
brew services list
brew services stop codex-mem9
```

## 7. 关于版本同步

`homebrew-tap` 仓库不需要和 `codex-mem9` 源码仓库使用相同的 git tag。

真正需要同步的是 Formula 中的这三个字段：

- `version`
- `url`
- `sha256`

也就是说：

- 源码仓库的 tag，例如 `v0.1.1`，是发布源头
- tap 仓库只需要把 Formula 更新到这个版本对应的 release 产物

因此：

- `codex-mem9` 的版本以源码仓库 tag 为准
- `homebrew-tap` 不需要额外打同版本 tag 才能工作
- 如果你愿意，tap 仓库可以完全不打 tag，只维护 `main` 分支上的 Formula

## 8. 每次发布检查清单

每次发布都按下面顺序执行：

1. 在源码仓库更新版本号
2. 在源码仓库执行本地校验
3. 提交并推送源码仓库到 `main`
4. 创建并推送源码仓库 tag `vX.Y.Z`
5. 等待 GitHub Release workflow 生成产物
6. 获取 release 产物地址和 SHA-256
7. 更新 tap 仓库中的 `Formula/codex-mem9.rb`
8. 推送 tap 仓库修改
9. 验证 `brew install codex-mem9`
10. 验证 `brew services start codex-mem9`
