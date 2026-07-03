/** Pure-TS unit tests for the JSON pass-through layer — mocks
 * `NativeModules.GenesisDb` so these run under plain Node, no RN runtime,
 * no simulator/emulator. See src/__tests__/index.test.ts. */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  roots: ['<rootDir>/src'],
  moduleNameMapper: {
    '^react-native$': '<rootDir>/src/__mocks__/react-native.ts',
  },
};
