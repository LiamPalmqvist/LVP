cask "notvlc" do
  version "0.1.0"
  sha256 "76982f3d104579beb7808ef27d2a22b118fc2b5a2f7fd6738c6bd601db9c33cc"

  url "https://github.com/LiamPalmqvist/NotVLC/releases/download/v#{version}/notvlc-#{version}-macos-arm64.zip"
  name "NotVLC"
  desc "Media player built on VLC's engine with a redesigned interface"
  homepage "https://github.com/LiamPalmqvist/NotVLC"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on :macos

  app "NotVLC.app"

  # The app is signed ad hoc, not notarized, so Gatekeeper would refuse the
  # first launch of a quarantined copy.
  postflight_steps do
    run "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "{{appdir}}/NotVLC.app"]
  end

  zap trash: [
    "~/Library/Application Support/org.NotVLC.NotVLC",
    "~/Library/Caches/org.NotVLC.NotVLC",
    "~/Library/Preferences/org.notvlc.NotVLC.plist",
  ]

  caveats <<~EOS
    NotVLC plays media through VLC's engine, so it needs VLC 3 in
    /Applications or ~/Applications. If you don't have it yet:
      brew install --cask vlc
  EOS
end
