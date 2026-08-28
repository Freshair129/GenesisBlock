require "json"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))

Pod::Spec.new do |s|
  s.name         = "react-native-genesisdb"
  s.version      = package["version"]
  s.summary      = package["description"]
  s.license      = package["license"]
  s.authors      = { "GenesisBlockDB" => "" }
  s.homepage     = "https://github.com/Freshair129/GenesisBlock"
  s.platforms    = { :ios => "13.0" }
  s.source       = { :git => "https://github.com/Freshair129/GenesisBlock.git" }
  s.source_files = "ios/**/*.{h,m,mm,swift}"

  # Required because this pod compiles Swift. Without it CocoaPods falls back to
  # the SWIFT_VERSION of whichever target integrates the pod, and fails outright
  # when no integrating target defines one:
  #
  #   [!] Unable to determine Swift version for the following pods:
  #   - `react-native-genesisdb` does not specify a Swift version and none of
  #     the targets integrating it have the `SWIFT_VERSION` attribute set.
  #
  # A stock RN app template happens to set it, which is why the omission stayed
  # invisible; a consumer whose target does not is simply blocked. 5.0 is the
  # language mode React Native's own pods declare, and ios/genesisdb builds with
  # swift-tools-version 5.9, which compiles 5.0 language mode.
  s.swift_version = "5.0"

  s.dependency "React-Core"

  # GenesisDbModule.swift now calls the real ios/genesisdb Swift package
  # (Phase B-1) instead of stub-rejecting every method. See
  # docs/SPEC--MOBILE-SDK.md §B-1/§B-3 and react-native-genesisdb/README.md
  # "iOS integration status".
  #
  # Correction from an earlier version of this comment (issue #125): getting
  # `GenesisBlockDB.xcframework` into a consumer's app via CocoaPods does NOT
  # require publishing this podspec to CocoaPods Trunk. RN autolinking's
  # `use_native_modules!` scans `node_modules/**/*.podspec` directly — this
  # file ships INSIDE the npm package and is picked up with zero Trunk publish,
  # the same way virtually every third-party RN native module distributes its
  # iOS half. (Trunk publish would only matter for a pure-iOS consumer doing
  # `pod 'react-native-genesisdb'` outside any RN project — not our audience.)
  #
  # CocoaPods still has no remote-URL binary-target mechanism the way SPM's
  # `.binaryTarget(url:, checksum:)` does, so the xcframework is fetched by a
  # `prepare_command` (a shell script CocoaPods runs in this directory before
  # `pod install` reads `s.vendored_frameworks`) that downloads + verifies +
  # unzips the same v0.2.0 release asset ios/README.md documents. The
  # URL/checksum are pinned to a specific release, NOT derived from
  # `package["version"]` (this package's own npm version, 0.1.0, doesn't move
  # in lockstep with the xcframework's release tag yet) — bump both together
  # by hand whenever a new xcframework is published.
  xcframework_zip_url = "https://github.com/Freshair129/GenesisBlock/releases/download/v0.2.0/GenesisBlockDB.xcframework.zip"
  xcframework_zip_sha256 = "607df0d82d68550a20927ae171928ad1decd7253fb647da450dec87deea1c26d"

  s.prepare_command = <<-CMD
    set -e
    if [ -d "GenesisBlockDB.xcframework" ]; then
      exit 0
    fi
    curl -fsSL "#{xcframework_zip_url}" -o GenesisBlockDB.xcframework.zip
    echo "#{xcframework_zip_sha256}  GenesisBlockDB.xcframework.zip" | shasum -a 256 -c -
    unzip -q -o GenesisBlockDB.xcframework.zip
    rm GenesisBlockDB.xcframework.zip
  CMD

  s.vendored_frameworks = "GenesisBlockDB.xcframework"

  # What this still does NOT solve: CocoaPods has no mechanism to depend on a
  # Swift Package (`ios/genesisdb`'s `GenesisDB`/`GenesisDBTypes` modules that
  # GenesisDbModule.swift imports) from inside a .podspec — that
  # cross-ecosystem dependency has to be added at the consuming Xcode project
  # level (Xcode's own "Add Package Dependency", pointing at `../ios/genesisdb`
  # for a monorepo checkout — issue #125 deliberately defers giving
  # `ios/genesisdb` its own root-level repo for a "real" published SPM URL),
  # alongside running `pod install` for this package. This is a real, standard
  # CocoaPods+SPM coexistence pattern, not a workaround.
end
