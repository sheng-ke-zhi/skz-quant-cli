class Skz < Formula
  desc "面向 AI Agent 的胜可知量化研究与实盘交易命令行工具"
  homepage "https://github.com/sheng-ke-zhi/skz-quant-cli"
  version "0.1.51"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.51/skz-aarch64-apple-darwin.tar.gz"
      sha256 "4534b43562de724c13df67d631580c0a56f83476f5bcc3456933e515440c35e7"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.51/skz-x86_64-apple-darwin.tar.gz"
      sha256 "8cbecea77a1b318d7eb8afd143a165dc57da934983aafce7c43db52b0f642bed"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.51/skz-aarch64-unknown-linux-musl.tar.gz"
      sha256 "40e49a08031e26e222d0e0703319f39face9e80661116679bc43cec7d69892ec"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.51/skz-x86_64-unknown-linux-musl.tar.gz"
      sha256 "00ab9f3a748cb75511fe889ff8ee82d07e750ce40b31377985fd84c8b507dbdf"
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
