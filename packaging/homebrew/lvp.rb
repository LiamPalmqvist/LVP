cask "lvp" do
  version "0.2.0"
  sha256 "b7da1b32c56e60e53e5e3a8bdbef64320387c85011e16e94b5bc7d27d7e8e2bd"

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
