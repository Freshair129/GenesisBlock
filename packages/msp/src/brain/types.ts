export type AtomType =
  | 'ADR'
  | 'FEAT'
  | 'BLUEPRINT'
  | 'AUDIT'
  | 'CONCEPT'
  | 'FRAMEWORK'
  | 'SPEC'
  | 'PROTOCOL'
  | 'SKILL'
  | 'ALGO'
  | 'PROTO'
  | 'PARAMS'
  | 'EPISODE'
  | 'IDENTITY'
  | 'REGISTRY'
  | 'MOD'
  | 'MASTER'
  | 'HOTFIX'
  | 'INC'
  | 'ISSUE';

export type BrainSource = 'global' | 'project';

export interface AtomRecord {
  id: string;
  type: AtomType;
}

export interface BrainQuery {
  id?: string;
  type?: AtomType;
  vault_id?: string;
}

export interface BrainHit {
  atom: AtomRecord;
  source: BrainSource;
  path: string;
}
