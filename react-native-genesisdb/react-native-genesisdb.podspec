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
  # "iOS integration status" for the two things NOT resolved by this podspec
  # alone (mirrors android/build.gradle's identical "not published yet"
  # caveat for genesisdb-android):
  #
  #   1. `GenesisBlockDB.xcframework` (the compiled C ABI, src/ffi.rs) IS now
  #      published as a v0.2.0 release asset:
  #      https://github.com/Freshair129/GenesisBlock/releases/download/v0.2.0/GenesisBlockDB.xcframework.zip
  #      (SHA256 8359846a8e668770816e0d84940aead0a85812f5aa67f91e7c2ff8308d37bc72)
  #      — but `s.vendored_frameworks` still needs a *local* path, not a URL;
  #      CocoaPods has no remote-binary-target mechanism the way SPM's
  #      `.binaryTarget(url:, checksum:)` does. Wiring this in means adding a
  #      `prepare_command` that downloads + unzips that URL during
  #      `pod install`, not yet written here.
  #   2. CocoaPods has no mechanism to depend on a Swift Package (`ios/
  #      genesisdb`'s `GenesisDB`/`GenesisDBTypes` modules that
  #      GenesisDbModule.swift imports) from inside a .podspec — that
  #      cross-ecosystem dependency has to be added at the consuming Xcode
  #      project level (Xcode's own "Add Package Dependency", pointing at
  #      `../ios/genesisdb` or its eventual published Git URL), alongside
  #      running `pod install` for this package. This is a real, standard
  #      CocoaPods+SPM coexistence pattern, not a workaround.
end
