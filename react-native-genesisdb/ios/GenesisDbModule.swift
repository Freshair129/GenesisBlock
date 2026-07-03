import Foundation

/// react-native-genesisdb iOS stub (MARK XVI Phase B-3).
///
/// The Android side of this package (../android) is a real, working bridge
/// over `dev.genesisblock:genesisdb-android` (Phase B-2). The iOS equivalent
/// depends on Phase B-1 (the `GenesisBlockDB.xcframework` + Swift wrapper
/// over `include/genesisdb.h`), which has not landed — see
/// docs/SPEC--MOBILE-SDK.md §B-1. This stub exists so `pod install` and RN
/// autolinking succeed on iOS today (the package doesn't fail to link just
/// because one platform isn't ready yet); every method rejects immediately
/// with a clear, actionable error instead of crashing at build/link time.
///
/// Once B-1 ships, replace each method body with a call into its Swift
/// `GenesisDB` actor, converting between the JSON-string bridge contract
/// used here (identical to the Android module's) and that actor's typed API.
@objc(GenesisDb)
class GenesisDbModule: NSObject {

    private static let pendingB1Message =
        "react-native-genesisdb has no iOS implementation yet (Phase B-1 — " +
        "the GenesisBlockDB.xcframework + Swift wrapper — has not landed; " +
        "see docs/SPEC--MOBILE-SDK.md §B-1). Android is available today."

    @objc static func requiresMainQueueSetup() -> Bool { false }

    private func rejectPending(_ reject: @escaping RCTPromiseRejectBlock) {
        reject("GENESISDB_IOS_NOT_IMPLEMENTED", GenesisDbModule.pendingB1Message, nil)
    }

    @objc
    func open(
        _ path: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        rejectPending(reject)
    }

    @objc
    func close(
        _ dbId: NSNumber,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        rejectPending(reject)
    }

    @objc
    func addNode(
        _ dbId: NSNumber,
        jsonInput: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        rejectPending(reject)
    }

    @objc
    func search(
        _ dbId: NSNumber,
        jsonInput: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        rejectPending(reject)
    }

    @objc
    func executeHql(
        _ dbId: NSNumber,
        query: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        rejectPending(reject)
    }

    @objc
    func retrieveContext(
        _ dbId: NSNumber,
        jsonInput: String,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        rejectPending(reject)
    }

    @objc
    func flushIndex(
        _ dbId: NSNumber,
        resolver resolve: @escaping RCTPromiseResolveBlock,
        rejecter reject: @escaping RCTPromiseRejectBlock
    ) {
        rejectPending(reject)
    }
}
