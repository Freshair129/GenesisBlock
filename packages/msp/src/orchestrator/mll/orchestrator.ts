import { distillSkillFromEpisodes } from './skill-creator.js'
import { calculateStabilityScore } from './stability.js'

export interface MllResult {
  skillsSuggested: string[]
  errors: string[]
}

/**
 * Heuristic to determine if a session should trigger meta-learning.
 */
function shouldRunMll(sessionId: string): boolean {
  // TODO: Read episode count for sessionId.
  // For now, we return true if session is not 'null'
  return sessionId !== 'null' && sessionId !== ''
}

/**
 * Top-level Meta-Learning Loop (MLL) Orchestrator.
 * 
 * Runs after a successful session to extract and formalize knowledge.
 * Implements BLUEPRINT--META-LEARNING-LOOP-SCAFFOLD.
 */
export async function runMll(opts: {
  root: string,
  sessionId: string,
  namespace: string,
  force?: boolean
}): Promise<MllResult> {
  const result: MllResult = {
    skillsSuggested: [],
    errors: []
  }

  try {
    console.log(`[mll] starting meta-learning loop for session ${opts.sessionId}`)

    // 1. Analyze session success and complexity (Trigger check)
    if (!opts.force && !shouldRunMll(opts.sessionId)) {
      console.log(`[mll] session ${opts.sessionId} does not meet complexity threshold. skipping.`)
      return result
    }
    
    // 2. Distill Skill Candidate
    // We first run a 'draft' distillation to get content for stability check.
    const initialSkills = await distillSkillFromEpisodes({
      root: opts.root,
      namespace: opts.namespace,
      limit: 5
    })

    if (initialSkills.length === 0) {
      return result
    }

    for (const skill of initialSkills) {
      // 3. Multi-Model Stability Check
      console.log(`[mll] checking stability for skill ${skill.skill_id}...`)
      const stabilityScore = await calculateStabilityScore(skill.body)

      // 4. Finalize and persist (re-run with stability score)
      // Note: In a production system, we'd just update the existing candidate.
      // For Phase 1, we re-run to ensure the frontmatter is correctly baked.
      await distillSkillFromEpisodes({
        root: opts.root,
        namespace: opts.namespace,
        stabilityScore,
        limit: 5
      })

      result.skillsSuggested.push(skill.skill_id)
    }

    console.log(`[mll] meta-learning loop complete. ${result.skillsSuggested.length} skills suggested.`)

  } catch (err) {
    const msg = (err as Error).message
    console.error(`[mll] error: ${msg}`)
    result.errors.push(msg)
  }

  return result
}
