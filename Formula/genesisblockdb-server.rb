class GenesisblockdbServer < Formula
  desc "Local-first semantic graph and vector database server"
  homepage "https://github.com/Freshair129/GenesisBlock"
  version "0.2.7"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/Freshair129/GenesisBlock/releases/download/v0.2.7/genesisblockdb-server-v0.2.7-aarch64-apple-darwin.tar.gz"
      sha256 "29084c25a11eff3923b101c5391b2f042ffe9cfe9ab818248c952a20ccc9f4f5"
    end
    on_intel do
      url "https://github.com/Freshair129/GenesisBlock/releases/download/v0.2.7/genesisblockdb-server-v0.2.7-x86_64-apple-darwin.tar.gz"
      sha256 "3174570f2929a86354c140f517b248bfd09ebb286c68e5fe470cc6da6e7c3178"
    end
  end

  on_linux do
    url "https://github.com/Freshair129/GenesisBlock/releases/download/v0.2.7/genesisblockdb-server-v0.2.7-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "979abab8546c8c1e9a8175ac531e22685982d394d0c8242859cbaaff32ee30a8"
  end

  def install
    binary = Dir["**/genesis-db-server"].first
    odie "release archive is missing genesis-db-server" unless binary

    bin.install binary
  end

  test do
    assert_predicate bin/"genesis-db-server", :executable?
  end
end
