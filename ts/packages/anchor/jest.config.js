module.exports = {
  preset: 'ts-jest/presets/default',
  testEnvironment: 'node',
  testTimeout: 90000,
  testMatch: ['**/*.test.ts'],
  transform: {
    '^.+\\.ts$': 'ts-jest',
  },
  resolver: "ts-jest-resolver",
};
