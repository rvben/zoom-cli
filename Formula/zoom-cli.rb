class ZoomCli < Formula
  desc "Agent-friendly Zoom CLI with JSON output, structured exit codes, and schema introspection"
  homepage "https://github.com/rvben/zoom-cli"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/rvben/zoom-cli/releases/download/v0.2.0/zoom-cli-v0.2.0-aarch64-apple-darwin.tar.gz"
      sha256 "1646d509545baaf0dad29c136f6d63a02d8537c0b2cf42b29c64b2777cca267b"
    end
    on_intel do
      url "https://github.com/rvben/zoom-cli/releases/download/v0.2.0/zoom-cli-v0.2.0-x86_64-apple-darwin.tar.gz"
      sha256 "94f11f7a6cbb59e1fe5e1b55e8518c52ed2774a1fbbfd7ede824f84258786c18"
    end
  end

  def install
    bin.install "zoom"
  end

  def caveats
    <<~EOS
      Run `zoom init` to set up your Zoom Server-to-Server OAuth credentials.
      Run `zoom config show` to verify your configuration.
    EOS
  end

  test do
    assert_match "zoom #{version}", shell_output("#{bin}/zoom --version")
  end
end
