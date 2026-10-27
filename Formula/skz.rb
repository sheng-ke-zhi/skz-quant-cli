class Skz < Formula
  desc "面向 AI Agent 的胜可知量化研究与实盘交易命令行工具"
  homepage "https://github.com/sheng-ke-zhi/skz-quant-cli"
  version "0.1.46"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.46/skz-aarch64-apple-darwin.tar.gz"
      sha256 "afaa69aa2764121919a5cde8764b4662e04e5fde24aab60727a012c2ffd89803"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.46/skz-x86_64-apple-darwin.tar.gz"
      sha256 "2f6c54c4c069f478d123d09faf60c77689e67aa317474eae95964b2806506dff"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.46/skz-aarch64-unknown-linux-musl.tar.gz"
      sha256 "1127a57d1d2fc80e8939a00bb846887e3800bf79292517b8d794d38ef3ffa145"
    end
    on_intel do
      url "https://github.com/sheng-ke-zhi/skz-quant-cli/releases/download/v0.1.46/skz-x86_64-unknown-linux-musl.tar.gz"
      sha256 "a5593f85b842cec7608c67de3ad099ec9589470e663b806d1facdfec3d5e6e30"
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
