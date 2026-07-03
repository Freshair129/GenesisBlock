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

  # NOTE: does not yet link GenesisBlockDB.xcframework (Phase B-1, not shipped
  # — see docs/SPEC--MOBILE-SDK.md §B-1). GenesisDbModule.swift is a stub that
  # rejects every call. Once B-1 lands, add:
  #   s.vendored_frameworks = "GenesisBlockDB.xcframework"
end
