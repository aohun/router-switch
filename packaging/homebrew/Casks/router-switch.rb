cask "router-switch" do
  arch arm: "aarch64-apple-darwin", intel: "x86_64-apple-darwin"

  version "0.1.2"
  sha256 arm:   "6c016e788bc273d4ee7e3dbdb6a0662d5a396e95fe76a59b6c0032b4b9b9c9f6",
         intel: "6c016e788bc273d4ee7e3dbdb6a0662d5a396e95fe76a59b6c0032b4b9b9c9f6"

  url "https://github.com/aohun/router-switch/releases/download/v#{version}/Router-Switch-#{version}-#{arch}.dmg",
      verified: "github.com/aohun/router-switch/"
  name "Router Switch"
  desc "Next-Generation AI Gateway Desktop Tool & Provider Hub"
  homepage "https://github.com/aohun/router-switch"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: ">= :big_sur"

  app "Router Switch.app"

  zap trash: [
    "~/.router-switch",
    "~/Library/Application Support/com.routerswitch.app",
    "~/Library/Caches/com.routerswitch.app",
    "~/Library/Preferences/com.routerswitch.app.plist",
    "~/Library/Saved Application State/com.routerswitch.app.savedState",
  ]
end
