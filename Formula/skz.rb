class Skz < Formula
  desc "面向 AI Agent 的胜可知量化研究与实盘交易命令行工具"
  homepage "https://github.com/sheng-ke-zhi/skz-quant-cli"
  version "0.1.48"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.48/skz-aarch64-apple-darwin.tar.gz"
      sha256 "0caa4d1163ec90f63dffb97897ced9625531763651cb8f3ed076d349620a9a6a"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.48/skz-x86_64-apple-darwin.tar.gz"
      sha256 "80b3cde4bd54aac29bbf2b65601b95a59d79348d56b8ae4db536a551ed05a806"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.48/skz-aarch64-unknown-linux-musl.tar.gz"
      sha256 "6609d8a5a99062e1cedba27c21eff3f94b0eb2ee8534e71de066905e8e3e91eb"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.48/skz-x86_64-unknown-linux-musl.tar.gz"
      sha256 "5c59b0d5b148e7a7fe39ffd39cf22c7b6915d5f72347f2fa23a42eedbbe3f9b3"
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
