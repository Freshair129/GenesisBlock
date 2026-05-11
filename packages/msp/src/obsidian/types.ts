export interface SearchHit {
  path: string
  title: string
  snippet: string
  score: number
}

export interface ClientOpts {
  root?: string
  url?: string
  apiKey?: string
  timeoutMs?: number
  fetch?: typeof fetch
  warn?: (msg: string) => void
}

export interface ObsidianClient {
  readonly mode: 'rest' | 'filesystem'
  search(query: string, opts?: { limit?: number }): Promise<SearchHit[]>
  readFile(relPath: string): Promise<string>
  activeFile?: () => Promise<string | null>
  smartViewDeepLink?: (atomId: string) => string
}

export interface ResolvedEnv {
  url: string | undefined
  apiKey: string | undefined
  insecure: boolean
  hostDeprecated: boolean
}
