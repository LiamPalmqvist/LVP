cask "lvp" do
  version "0.1.0"
  sha256 "76982f3d104579beb7808ef27d2a22b118fc2b5a2f7fd6738c6bd601db9c33cc"

  url "https://github.com/LiamPalmqvist/LVP/releases/download/v#{version}/lvp-#{version}-macos-arm64.zip"
  name "LVP"
  desc "Media player built on VLC's engine with a redesigned interface"
  homepage "https://github.com/LiamPalmqvist/LVP"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on :macos

  app "LVP.app"

  # The app is signed ad hoc, not notarized, so Gatekeeper would refuse the
  # first launch of a quarantined copy.
  postflight_steps do
    run "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "{{appdir}}/LVP.app"]
  end

  zap trash: [
    "~/Library/Application Support/org.LVP.LVP",
    "~/Library/Caches/org.LVP.LVP",
    "~/Library/Preferences/org.lvp.LVP.plist",
  ]
end
