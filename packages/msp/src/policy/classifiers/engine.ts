import type { AttributeBag } from '../types.js'
import type { ClassifiableResource, ClassificationResult, Classifier, Provenance } from './types.js'

/**
 * Orchestrates multiple classifiers and merges their outputs.
 * Implements precedence rules: Frontmatter > Domain Pack > Universal.
 */
export async function runClassifiers(
  resource: ClassifiableResource,
  classifiers: Classifier[]
): Promise<ClassificationResult> {
  const mergedAttributes: AttributeBag = { ...(resource.attributes ?? {}) }
  const mergedProvenance: Record<string, Provenance> = {}

  // 1. Tag existing attributes as 'manual/frontmatter'
  for (const key of Object.keys(mergedAttributes)) {
    mergedProvenance[key] = {
      classifier_id: 'manual/frontmatter',
      timestamp: new Date().toISOString(),
      confidence: 1.0,
    }
  }

  // 2. Run each classifier
  for (const classifier of classifiers) {
    try {
      const result = await classifier.classify(resource)
      
      for (const [key, value] of Object.entries(result.attributes)) {
        const prov = result.provenance[key] || {
          classifier_id: classifier.id,
          timestamp: new Date().toISOString(),
        }

        // PRECEDENCE RULE: Don't overwrite higher-priority sources.
        // Priority: manual > classifier
        const existingProv = mergedProvenance[key]
        if (existingProv && existingProv.classifier_id === 'manual/frontmatter') {
          // Keep manual tag
          continue
        }

        mergedAttributes[key] = value
        mergedProvenance[key] = prov
      }
    } catch (err) {
      console.warn(`[policy] classifier ${classifier.id} failed: ${(err as Error).message}`)
    }
  }

  return {
    attributes: mergedAttributes,
    provenance: mergedProvenance,
  }
}
