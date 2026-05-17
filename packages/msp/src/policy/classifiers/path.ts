import { dirname, sep } from 'node:path'
import type { ClassifiableResource, ClassificationResult, Classifier } from './types.js'

/**
 * Tags 'domain' based on the file path.
 * E.g. 'gks/adr/ADR--1.md' -> domain: 'adr'
 */
export class PathClassifier implements Classifier {
  readonly id = 'universal/path'
  readonly description = 'Derives domain from directory structure'
  readonly outputs = ['domain']

  async classify(resource: ClassifiableResource): Promise<ClassificationResult> {
    const attributes: Record<string, any> = {}
    
    // Use the directory name as the domain
    const dir = dirname(resource.path)
    const segments = dir.split(/[\\/]/).filter(s => s !== '.' && s !== '..')
    
    if (segments.length > 0) {
      // Last segment is usually the most specific domain
      // e.g. gks/adr -> adr
      // e.g. packages/msp/src/policy -> policy
      const domain = segments[segments.length - 1]
      if (domain && domain !== 'gks' && domain !== 'src') {
        attributes.domain = domain
      }
    }

    return {
      attributes,
      provenance: {
        domain: {
          classifier_id: this.id,
          timestamp: new Date().toISOString(),
          confidence: 0.9,
        }
      }
    }
  }
}
