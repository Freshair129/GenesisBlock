import { NativeModules, Platform } from 'react-native';

/**
 * The native module's `dbId` is an opaque small integer minted by the
 * platform module (Android: `GenesisDbModule`'s own counter; iOS: same
 * pattern once B-1 lands) — it is NOT the raw native pointer/handle. Sending
 * a real 64-bit pointer across the RN bridge as a JS `number` risks silent
 * precision loss above 2^53, so the platform side keeps the actual handle
 * behind this id and this package never sees it.
 */
export interface GenesisDbNativeModule {
  open(path: string): Promise<number>;
  close(dbId: number): Promise<void>;
  addNode(dbId: number, jsonInput: string): Promise<string>;
  search(dbId: number, jsonInput: string): Promise<string>;
  executeHql(dbId: number, query: string): Promise<string>;
  retrieveContext(dbId: number, jsonInput: string): Promise<string>;
  flushIndex(dbId: number): Promise<void>;
}

const LINKING_ERROR =
  `The package 'react-native-genesisdb' doesn't seem to be linked. Make sure: \n\n` +
  Platform.select({ ios: "- You have run 'pod install'\n", default: '' }) +
  '- You rebuilt the app after installing the package\n' +
  '- You are not using Expo Go\n';

const GenesisDb: GenesisDbNativeModule = NativeModules.GenesisDb
  ? NativeModules.GenesisDb
  : new Proxy(
      {},
      {
        get() {
          throw new Error(LINKING_ERROR);
        },
      }
    );

export default GenesisDb;
