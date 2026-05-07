import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: ['test/**/*.test.ts'],
    environment: 'node',
    // test/scripts/propose.test.ts writes to repoRoot/.brain/msp/projects/evaAI/inbound,
    // which test/validator/cli.test.ts walks via --all. Run files serially to avoid
    // the race that flakes CI on slower runners (intermittent on Node 20 and 22).
    fileParallelism: false,
  },
})
