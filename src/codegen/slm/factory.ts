import type { SlmCall, SlmClient } from '../types.js'
import { createOllamaClient } from './ollama.js'
import { SlmError } from './errors.js'
import type { SlmFactoryOpts } from './types.js'

const mockClient: SlmClient = async ({ prompt }: SlmCall) => {
  const m = prompt.match(/MOCK_OUTPUT:\s*([\s\S]+?)(?:\n##|$)/)
  if (m) return m[1]!.trim() + '\n'
  return 'export const handler = async () => ({ ok: true })\n'
}

export function createSlmClient(opts: SlmFactoryOpts = {}): SlmClient {
  const provider =
    opts.provider ??
    (process.env.MSP_SLM_PROVIDER as SlmFactoryOpts['provider']) ??
    'ollama'

  switch (provider) {
    case 'ollama':
      return createOllamaClient(opts.ollama ?? {})
    case 'mock':
      return mockClient
    default:
      throw new SlmError(`unknown SLM provider '${provider}'`, 'config')
  }
}

export { createOllamaClient } from './ollama.js'
export { SlmError, type SlmErrorKind } from './errors.js'
export type { OllamaOpts, SlmFactoryOpts } from './types.js'
