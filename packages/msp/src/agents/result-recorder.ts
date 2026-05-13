import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'

import type { DispatchResult, DispatchTask } from './types.js'

/**
 * Best-effort episodic recorder for dispatcher runs.
 *
 * Writes one markdown atom per dispatch outcome under
 *   <root>/gks/episode/EPISODE--AGENT-RUN-<isoTimestamp>.md
 *
 * NOTE: the validator does NOT currently recognise `type: 'episode'` as a
 * strict tracked type. We pick the pragmatic frontmatter below and rely on the
 * file being a useful artefact rather than a fully-validated atom. If validator
 * coverage is later added for episodes, this frontmatter should be revisited.
 * (TODO: SPEC for EPISODE atoms — out of scope for BLUEPRINT--AGENT-DISPATCHER P4.)
 *
 * Failures (mkdir/writeFile errors) propagate — dispatch.ts wraps the call in
 * try/catch and treats this as best-effort.
 */
export async function recordEpisode(
  task: DispatchTask,
  result: DispatchResult,
  root: string,
): Promise<string> {
  const createdAt = new Date().toISOString()
  // File-safe ISO timestamp (no `:` — Windows-hostile).
  const stamp = createdAt.replace(/[:.]/g, '-')
  const id = `EPISODE--AGENT-RUN-${stamp}`
  const filename = `${id}.md`
  const absDir = resolve(root, 'gks', 'episode')
  const absPath = join(absDir, filename)

  const titleSnippet = task.prompt.slice(0, 60).replace(/\s+/g, ' ').trim()
  const title = `EPISODE — Agent run ${stamp} (${result.tier_used})`

  const tags: readonly string[] = ['agents', 'dispatch', result.tier_used.toLowerCase()]

  const escalatedLine = result.escalated_from
    ? `escalated_from: ${result.escalated_from}\n`
    : ''
  const costLine =
    typeof result.cost_usd === 'number' ? `cost_usd: ${result.cost_usd}\n` : ''

  const frontmatter = [
    '---',
    `id: ${id}`,
    'phase: 5',
    "type: episode",
    "status: stable",
    "tier: genesis",
    "source_type: episodic",
    "vault_id: default",
    `title: ${escapeYaml(title)}`,
    'tags:',
    ...tags.map((t) => `  - ${t}`),
    `created_at: ${createdAt}`,
    '---',
    '',
  ].join('\n')

  const body = [
    `# ${title}`,
    '',
    `**Prompt (truncated):** ${escapeMd(titleSnippet)}`,
    '',
    `- tier_used: ${result.tier_used}`,
    result.escalated_from ? `- escalated_from: ${result.escalated_from}` : null,
    `- duration_ms: ${result.duration_ms}`,
    typeof result.cost_usd === 'number' ? `- cost_usd: ${result.cost_usd}` : null,
    `- task.type: ${task.type}`,
    `- task.severity: ${task.severity}`,
    typeof task.context_size_tokens === 'number'
      ? `- context_size_tokens: ${task.context_size_tokens}`
      : null,
    '',
    '## Prompt',
    '',
    '```',
    task.prompt,
    '```',
    '',
    '## Output',
    '',
    '```',
    result.output,
    '```',
    '',
  ]
    .filter((line): line is string => line !== null)
    .join('\n')

  // Silence unused-var warnings — these fields are baked into frontmatter/body above.
  void escalatedLine
  void costLine

  await mkdir(dirname(absPath), { recursive: true })
  await writeFile(absPath, frontmatter + body, 'utf8')
  return absPath
}

function escapeYaml(s: string): string {
  // Quote the title to avoid YAML parsing on `:` etc.
  return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`
}

function escapeMd(s: string): string {
  return s.replace(/`/g, 'ˋ')
}
