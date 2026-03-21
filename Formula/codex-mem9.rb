class CodexMem9 < Formula
  desc "Sync and watch Codex memories into Mem9 with redaction"
  homepage "https://github.com/dmego/codex-mem9"
  url "https://github.com/dmego/codex-mem9/archive/refs/tags/v0.1.4.tar.gz"
  version "0.1.4"
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

  def caveats
    <<~EOS
      Homebrew installs the `codex-mem9` binary and service only.
      Install the skills from the repository separately before you try to run
      the `mem9-setup` skill.

      This Formula installs the published release referenced above (#{version}).
      The installed behavior follows that release tag, not unreleased repository changes.

      Review the README that matches the same published release before configuring
      `brew services`, because configuration behavior can change between tags.
    EOS
  end

  test do
    output = shell_output("#{bin}/codex-mem9 sync 2>&1", 1)
    assert_match "missing MEM9_TENANT_ID", output
  end
end
