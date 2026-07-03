// Objective-C bridge exposing the Swift GenesisDbModule to RN's (old
// architecture) bridge — RCT_EXTERN_MODULE/RCT_EXTERN_METHOD can only be
// written in Objective-C, so Swift-authored RN modules always pair with a
// thin .m file like this one. Selector names below must match the Swift
// method signatures in GenesisDbModule.swift exactly (Swift derives the
// Objective-C selector from each method's external parameter labels).
#import <React/RCTBridgeModule.h>

@interface RCT_EXTERN_MODULE(GenesisDb, NSObject)

RCT_EXTERN_METHOD(open:(NSString *)path
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject)

RCT_EXTERN_METHOD(close:(nonnull NSNumber *)dbId
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject)

RCT_EXTERN_METHOD(addNode:(nonnull NSNumber *)dbId
                  jsonInput:(NSString *)jsonInput
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject)

RCT_EXTERN_METHOD(search:(nonnull NSNumber *)dbId
                  jsonInput:(NSString *)jsonInput
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject)

RCT_EXTERN_METHOD(executeHql:(nonnull NSNumber *)dbId
                  query:(NSString *)query
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject)

RCT_EXTERN_METHOD(retrieveContext:(nonnull NSNumber *)dbId
                  jsonInput:(NSString *)jsonInput
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject)

RCT_EXTERN_METHOD(flushIndex:(nonnull NSNumber *)dbId
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject)

@end
