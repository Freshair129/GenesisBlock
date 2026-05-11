import type { SessionTurn } from '../sessions/types.js'

export interface EpisodeContent {
  summary: string
  key_decisions?: string[]
  unresolved_questions?: string[]
}

export interface EpisodeAnchor {
  content: string
  msgId: string
}

export interface EpisodeContext {
  topic?: string
  participants?: string[]
  mood?: string
}

export interface Associations {
  related_event_ids?: string[]
  entity_links?: string[]
  knowledgeId?: string
  learnId?: string
}

export interface Episode {
  episodicId: string
  sessionId: string
  projectId: string
  timestamp?: string
  importance_score: number
  range: string[]
  anchor?: EpisodeAnchor
  context?: EpisodeContext
  content: EpisodeContent
  tags?: string[]
  associations?: Associations
}

export interface AppendOpts {
  root: string
  namespace?: string
}

export interface FromTurnsOpts extends AppendOpts {
  episodicId: string
  sessionId: string
  projectId: string
  range: string[]
  importance_score: number
  summariser?: Summariser
  context?: EpisodeContext
  tags?: string[]
  associations?: Associations
}

export type Summariser = (turns: SessionTurn[]) => Promise<EpisodeContent>

export class EpisodicSchemaError extends Error {
  constructor(public readonly reason: string) {
    super(`episodic schema: ${reason}`)
    this.name = 'EpisodicSchemaError'
  }
}
