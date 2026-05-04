import { z } from 'zod'

/**
 * Voice / style descriptors that shape how the agent speaks. The agent reads
 * these on session start and can refresh them across turns.
 *
 * `tone` is an ordered list — earlier items dominate. `language_preference`
 * is free-form (e.g. `"thai+english"`, `"en-US"`) so projects can encode
 * mixed-language conventions without a fixed enum.
 */
export const VoiceSchema = z
  .object({
    tone: z.array(z.string().min(1)).default([]),
    language_preference: z.string().min(1).optional(),
    formality: z.string().min(1).optional(),
    response_cadence: z.string().min(1).optional(),
    vocabulary_preferences: z.array(z.string().min(1)).optional(),
  })
  .strict()

export type Voice = z.infer<typeof VoiceSchema>

export function emptyVoice(): Voice {
  return { tone: [] }
}

/** Shallow merge — `patch` fields overwrite, undefined fields keep prior. */
export function mergeVoice(prior: Voice | undefined, patch: Partial<Voice>): Voice {
  const base = prior ?? emptyVoice()
  return VoiceSchema.parse({
    ...base,
    ...Object.fromEntries(Object.entries(patch).filter(([, v]) => v !== undefined)),
  })
}
