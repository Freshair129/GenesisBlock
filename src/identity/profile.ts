import { mkdir, readFile, rename, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

import { parse as parseYaml, stringify as stringifyYaml } from 'yaml'
import { z } from 'zod'

import { PreferencesSchema, type Preferences } from './preferences.js'
import { VoiceSchema, type Voice } from './voice.js'

export const DEFAULT_NAMESPACE = 'evaAI'

export const IdentityProfileSchema = z
  .object({
    name: z.string().min(1).optional(),
    voice: VoiceSchema.optional(),
    preferences: PreferencesSchema.optional(),
    origin_story: z.string().optional(),
  })
  .strict()

export type IdentityProfile = z.infer<typeof IdentityProfileSchema>

export class IdentityParseError extends Error {
  constructor(public readonly path: string, cause: unknown) {
    super(`identity.yaml at ${path} is invalid: ${(cause as Error).message}`)
    this.name = 'IdentityParseError'
  }
}

export function identityFilePath(root: string, namespace: string): string {
  return resolve(root, '.brain/msp/projects', namespace, 'identity.yaml')
}

/** Returns null if the file does not exist; throws on parse/schema error. */
export async function loadProfile(
  root: string,
  namespace: string = DEFAULT_NAMESPACE,
): Promise<IdentityProfile | null> {
  const path = identityFilePath(root, namespace)
  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return null
    throw err
  }

  let parsed: unknown
  try {
    parsed = parseYaml(raw)
  } catch (err) {
    throw new IdentityParseError(path, err)
  }
  if (parsed === null || parsed === undefined) return {}

  try {
    return IdentityProfileSchema.parse(parsed)
  } catch (err) {
    throw new IdentityParseError(path, err)
  }
}

/**
 * Atomically replace the identity file. Validates against the schema before
 * writing so a malformed profile cannot land on disk.
 */
export async function saveProfile(
  root: string,
  namespace: string,
  profile: IdentityProfile,
): Promise<void> {
  const validated = IdentityProfileSchema.parse(profile)
  const path = identityFilePath(root, namespace)
  await mkdir(dirname(path), { recursive: true })
  const tmp = `${path}.tmp.${process.pid}.${Date.now()}`
  const yaml = stringifyYaml(validated)
  await writeFile(tmp, yaml, 'utf8')
  await rename(tmp, path)
}

export function emptyProfile(): IdentityProfile {
  return {}
}

/**
 * Merge a partial profile onto an existing one. `voice` and `preferences`
 * merge field-by-field; everything else replaces wholesale. Undefined fields
 * in `patch` leave the prior value untouched.
 */
export function mergeProfile(
  prior: IdentityProfile,
  patch: Partial<IdentityProfile>,
): IdentityProfile {
  const next: IdentityProfile = { ...prior }
  if (patch.name !== undefined) next.name = patch.name
  if (patch.origin_story !== undefined) next.origin_story = patch.origin_story
  if (patch.voice !== undefined) {
    next.voice = { ...(prior.voice ?? { tone: [] }), ...patch.voice } as Voice
  }
  if (patch.preferences !== undefined) {
    next.preferences = { ...(prior.preferences ?? {}), ...patch.preferences } as Preferences
  }
  return IdentityProfileSchema.parse(next)
}
