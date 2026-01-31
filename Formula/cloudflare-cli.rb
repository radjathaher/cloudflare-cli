class CloudflareCli < Formula
  desc "Cloudflare CLI"
  homepage "https://github.com/radjathaher/cloudflare-cli"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/radjathaher/cloudflare-cli/releases/download/v0.1.0/cloudflare-cli-0.1.0-darwin-aarch64.tar.gz"
      sha256 "f966a29c56e2773c0047d9d28004568ad0f9c68ff1a9143ac0ed8d2f015701e0"
    else
      odie "cloudflare-cli is only packaged for macOS arm64"
    end
  end

  def install
    bin.install "cloudflare"
  end
end
