import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { migrateIfNeeded } from '../../src/identity/migrate.js'
import { readIdentity } from '../../src/identity/store.js'
import { defaultIdentity } from '../../src/identity/types.js'
import { globalIdentityPath } from '../../src/lib/msp-home.js'

async function freshRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'msp-identity-migrate-'))
  process.env['MSP_HOME'] = resolve(root, '.msp')
  return root
}

let savedHome: string | undefined
beforeEach(() => {
  savedHome = process.env['MSP_HOME']
})
afterEach(() => {
  if (savedHome === undefined) delete process.env['MSP_HOME']
  else process.env['MSP_HOME'] = savedHome
})

const SAMPLE_LEGACY = {
  schemaVersion: 1,
  profile: {
    name: 'legacy',
    role: 'history',
    tier: 'T2' as const,
    originStory: 'born in workspace',
    createdAt: '2025-01-01T00:00:00.000Z',
    guardrails: ['old rule'],
    extensions: { 'pre/migration': true },
  },
  voice: {
    tone: ['archival'],
    formality: 'formal' as const,
    languagePreference: 'en',
    responseCadence: 'normal' as const,
  },
  preferences: { topK: { value: 7, expiresAt: null } },
}

describe('migrateIfNeeded', () => {
  it('migrates workspace → global when global missing + workspace exists', async () => {
    const root = await freshRoot()
    const legacyDir = join(root, '.brain/msp/projects/evaAI')
    await mkdir(legacyDir, { recursive: true })
    const legacyPath = join(legacyDir, 'identity.json')
    await writeFile(legacyPath, JSON.stringify(SAMPLE_LEGACY), 'utf8')

    const result = await migrateIfNeeded(root, 'evaAI')

    expect(result.migrated).toBe(true)
    expect(result.source).toBe(legacyPath)
    expect(result.destination).toBe(globalIdentityPath())
    expect(existsSync(globalIdentityPath())).toBe(true)
    // Legacy file is preserved (not deleted) per ADR
    expect(existsSync(legacyPath)).toBe(true)
    // Content matches
    const globalContent = JSON.parse(
      await readFile(globalIdentityPath(), 'utf8'),
    )
    expect(globalContent.profile.name).toBe('legacy')
  })

  it('noop when global already exists', async () => {
    await freshRoot()
    const globalPath = globalIdentityPath()
    await mkdir(dirname(globalPath), { recursive: true })
    const id = defaultIdentity()
    id.profile.name = 'existing-global'
    await writeFile(globalPath, JSON.stringify(id), 'utf8')

    // Even if a workspace identity also exists, do not overwrite the global.
    const root = process.env['MSP_HOME']!.replace(/\.msp$/, '')
    const legacyDir = join(root, '.brain/msp/projects/evaAI')
    await mkdir(legacyDir, { recursive: true })
    await writeFile(
      join(legacyDir, 'identity.json'),
      JSON.stringify(SAMPLE_LEGACY),
      'utf8',
    )

    const result = await migrateIfNeeded(root, 'evaAI')
    expect(result.migrated).toBe(false)
    expect(result.source).toBeUndefined()
    const stillGlobal = JSON.parse(
      await readFile(globalIdentityPath(), 'utf8'),
    )
    expect(stillGlobal.profile.name).toBe('existing-global')
  })

  it('noop when neither file exists (fresh install)', async () => {
    const root = await freshRoot()
    const result = await migrateIfNeeded(root, 'evaAI')
    expect(result.migrated).toBe(false)
    expect(existsSync(globalIdentityPath())).toBe(false)
  })

  it('is idempotent — second call after migration is a noop', async () => {
    const root = await freshRoot()
    const legacyDir = join(root, '.brain/msp/projects/evaAI')
    await mkdir(legacyDir, { recursive: true })
    await writeFile(
      join(legacyDir, 'identity.json'),
      JSON.stringify(SAMPLE_LEGACY),
      'utf8',
    )

    const first = await migrateIfNeeded(root, 'evaAI')
    expect(first.migrated).toBe(true)
    const second = await migrateIfNeeded(root, 'evaAI')
    expect(second.migrated).toBe(false)
  })

  it('refuses to migrate a legacy file with schemaVersion > 1', async () => {
    const root = await freshRoot()
    const legacyDir = join(root, '.brain/msp/projects/evaAI')
    await mkdir(legacyDir, { recursive: true })
    await writeFile(
      join(legacyDir, 'identity.json'),
      JSON.stringify({ schemaVersion: 99, profile: {}, voice: {}, preferences: {} }),
      'utf8',
    )

    const result = await migrateIfNeeded(root, 'evaAI')
    expect(result.migrated).toBe(false)
    expect(existsSync(globalIdentityPath())).toBe(false)
  })

  it('refuses to migrate a corrupt JSON legacy file', async () => {
    const root = await freshRoot()
    const legacyDir = join(root, '.brain/msp/projects/evaAI')
    await mkdir(legacyDir, { recursive: true })
    await writeFile(
      join(legacyDir, 'identity.json'),
      'not-valid-json{',
      'utf8',
    )
    const result = await migrateIfNeeded(root, 'evaAI')
    expect(result.migrated).toBe(false)
    expect(existsSync(globalIdentityPath())).toBe(false)
  })
})

describe('readIdentity migration integration', () => {
  it('upgrades workspace identity to global on first read', async () => {
    const root = await freshRoot()
    const legacyDir = join(root, '.brain/msp/projects/evaAI')
    await mkdir(legacyDir, { recursive: true })
    const legacyPath = join(legacyDir, 'identity.json')
    await writeFile(legacyPath, JSON.stringify(SAMPLE_LEGACY), 'utf8')

    expect(existsSync(globalIdentityPath())).toBe(false)
    const id = await readIdentity({ root, namespace: 'evaAI' })

    // Returned identity matches legacy content
    expect(id.profile.name).toBe('legacy')
    expect(id.profile.tier).toBe('T2')
    expect(id.voice.formality).toBe('formal')
    expect(id.preferences.topK.value).toBe(7)
    // Global file was created, legacy was preserved
    expect(existsSync(globalIdentityPath())).toBe(true)
    expect(existsSync(legacyPath)).toBe(true)
  })

  it('MSP_DISABLE_MIGRATION=1 skips migration even when legacy is present', async () => {
    const root = await freshRoot()
    const legacyDir = join(root, '.brain/msp/projects/evaAI')
    await mkdir(legacyDir, { recursive: true })
    await writeFile(
      join(legacyDir, 'identity.json'),
      JSON.stringify(SAMPLE_LEGACY),
      'utf8',
    )

    const oldDisable = process.env['MSP_DISABLE_MIGRATION']
    process.env['MSP_DISABLE_MIGRATION'] = '1'
    try {
      const id = await readIdentity({ root, namespace: 'evaAI' })
      // No migration → no global → defaults
      expect(id.profile.name).toBe('')
      expect(existsSync(globalIdentityPath())).toBe(false)
    } finally {
      if (oldDisable === undefined) delete process.env['MSP_DISABLE_MIGRATION']
      else process.env['MSP_DISABLE_MIGRATION'] = oldDisable
    }
  })

  it('merged read: global has voice.formality=casual, project override has formal → merged returns formal', async () => {
    const root = await freshRoot()
    // Pre-create the global directly (bypass migration).
    const globalPath = globalIdentityPath()
    await mkdir(dirname(globalPath), { recursive: true })
    const global = defaultIdentity()
    global.voice.formality = 'casual'
    global.voice.tone = ['warm']
    await writeFile(globalPath, JSON.stringify(global), 'utf8')

    // Project override at clinic
    const overrideDir = join(root, '.brain/msp/projects/clinic')
    await mkdir(overrideDir, { recursive: true })
    await writeFile(
      join(overrideDir, 'identity.override.json'),
      JSON.stringify({ voice: { ...global.voice, formality: 'formal' } }),
      'utf8',
    )

    const merged = await readIdentity({ root, namespace: 'clinic' })
    expect(merged.voice.formality).toBe('formal')
    // Tone falls back to override's voice (which copied from global)
    expect(merged.voice.tone).toEqual(['warm'])
    // Other namespace sees only global
    const evaAI = await readIdentity({ root, namespace: 'evaAI' })
    expect(evaAI.voice.formality).toBe('casual')
  })
})
