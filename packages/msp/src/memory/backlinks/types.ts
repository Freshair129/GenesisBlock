export interface Edge {
  from: string
  to: string
  type: string
}

export interface RebuildOpts {
  root: string
  namespace?: string
  dryRun?: boolean
  check?: boolean
}

export interface RebuildResult {
  atomCount: number
  edgeCount: number
  changed: boolean
  outputPath: string
}
