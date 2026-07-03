/** Minimal `react-native` stub so src/index.ts and NativeGenesisDb.ts
 * type-check and run under plain Jest/Node — no RN runtime involved. Tests
 * populate `NativeModules.GenesisDb` themselves before importing the module
 * under test. */
export const NativeModules: Record<string, unknown> = {};

export const Platform = {
  OS: 'android' as const,
  select: <T>(spec: { ios?: T; android?: T; default?: T }): T | undefined =>
    spec.android ?? spec.default,
};
