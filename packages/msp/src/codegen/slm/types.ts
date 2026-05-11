export type { SlmClient, SlmCall } from '../types.js'
export { SlmError, type SlmErrorKind } from './errors.js'

export interface OllamaOpts {
  host?: string
  model?: string
  fetchImpl?: typeof fetch
  timeoutMs?: number
  temperature?: number
}

export interface QwenOpts {
  path?: string
  model?: string
  temperature?: number
}

export interface SlmFactoryOpts {
  provider?: 'ollama' | 'mock' | 'qwen'
  ollama?: OllamaOpts
  qwen?: QwenOpts
}
