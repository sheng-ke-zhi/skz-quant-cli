class Skz < Formula
  desc "面向 AI Agent 的胜可知量化研究与实盘交易命令行工具"
  homepage "https://github.com/sheng-ke-zhi/skz-quant-cli"
  version "0.1.52"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.52/skz-aarch64-apple-darwin.tar.gz"
      sha256 "4bfeb5d31d69c77e4e6fb7f3e5d9b71200df0e602623ab6edd0b82696b481c65"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.52/skz-x86_64-apple-darwin.tar.gz"
      sha256 "4b2e6f095a204456ea4bbba5cc7d8caee9fbdf85d43df4caaf0aad9f0f442432"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.52/skz-aarch64-unknown-linux-musl.tar.gz"
      sha256 "b3f0d0211dc9a14aefe6a28f4f9370ad0f346646bcae597c9428c79931424b05"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.52/skz-x86_64-unknown-linux-musl.tar.gz"
      sha256 "c3b7e8dd90c3ea47a5a5a3e90a9dfac1f99d55f13132df6a753d52d83289b64b"
    end
  end

  def install
    libexec.install "skz", "plugins"
    bin.install_symlink libexec/"skz"
  end

  test do
    system "#{bin}/skz", "--version"
  end
end
