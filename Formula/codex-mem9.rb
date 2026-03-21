class CodexMem9 < Formula
  desc "Sync and watch Codex memories into Mem9 with redaction"
  homepage "https://github.com/dmego/codex-mem9"
  url "https://github.com/dmego/codex-mem9/archive/refs/tags/v0.1.0.tar.gz"
  version "0.1.0"
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
