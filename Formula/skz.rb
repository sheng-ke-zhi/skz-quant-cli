class Skz < Formula
  desc "面向 AI Agent 的胜可知量化研究与实盘交易命令行工具"
  homepage "https://github.com/sheng-ke-zhi/skz-quant-cli"
  version "0.1.49"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.49/skz-aarch64-apple-darwin.tar.gz"
      sha256 "2075156aa1a610370913fdc4c4012a08f77c842c7a16252cfee22f405c650824"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.49/skz-x86_64-apple-darwin.tar.gz"
      sha256 "b98cf01f5bb1346de99a3c54337a1a38663f1f72f4a1001a404290afd2e51f1c"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.49/skz-aarch64-unknown-linux-musl.tar.gz"
      sha256 "fc182e9da8ce986072e93489b6e35d28af0e32a9c8c7b46e77a3f288930d1307"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.49/skz-x86_64-unknown-linux-musl.tar.gz"
      sha256 "057a8b4ab435ed279b0526cd1d716bee0d9c2463b5166ba6d32e39f99617861a"
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
