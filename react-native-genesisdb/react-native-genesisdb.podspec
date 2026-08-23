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
  #   1. `GenesisBlockDB.xcframework` (the compiled C ABI, src/ffi.rs) is not
  #      yet published as a release asset — until it is, there is nothing
  #      for `s.vendored_frameworks` to point at, so it is deliberately
  #      absent below. A local monorepo build can assemble one itself:
  #      `.github/workflows/mobile-build.yml`'s `ios-xcframework` job shows
  #      the exact `xcodebuild -create-xcframework` invocation.
  #   2. CocoaPods has no mechanism to depend on a Swift Package (`ios/
  #      genesisdb`'s `GenesisDB`/`GenesisDBTypes` modules that
  #      GenesisDbModule.swift imports) from inside a .podspec — that
  #      cross-ecosystem dependency has to be added at the consuming Xcode
  #      project level (Xcode's own "Add Package Dependency", pointing at
  #      `../ios/genesisdb` or its eventual published Git URL), alongside
  #      running `pod install` for this package. This is a real, standard
  #      CocoaPods+SPM coexistence pattern, not a workaround.
  #
  # Once (1) is published, add `s.vendored_frameworks = "GenesisBlockDB.xcframework"`
  # here so the C ABI itself is included without a manual local build step.
end
