class CodexMem9 < Formula
  desc "Sync and watch Codex memories into Mem9 with redaction"
  homepage "https://github.com/dmego/codex-mem9"
  url "https://github.com/dmego/codex-mem9/archive/refs/tags/v0.1.1.tar.gz"
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

  def caveats
    <<~EOS
      For `brew services`, launchd does not load your interactive shell profile.

      Configure Mem9 for the service with either:
        1. #{Dir.home}/Library/Application Support/ai.dmego.codex-mem9/config.toml
           tenant_id = "<your-tenant-id>"
           api_url = "https://api.mem9.ai"
        2. launchctl setenv MEM9_TENANT_ID "<your-tenant-id>"
           launchctl setenv MEM9_API_URL "https://api.mem9.ai"
           brew services restart codex-mem9

      If your Mem9 deployment requires it, also set MEM9_API_KEY in the same place.
    EOS
  end

  test do
    assert_match "sync", shell_output("#{bin}/codex-mem9 --help")
  end
end
