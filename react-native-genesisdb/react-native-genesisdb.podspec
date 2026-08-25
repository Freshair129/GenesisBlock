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
  xcframework_zip_sha256 = "a4d2b0f267a15c1b8b82c349655b0fe2bc521fd2b1905c7c2bd6714e3f8db97f"

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
