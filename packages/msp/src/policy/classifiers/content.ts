import type { ClassifiableResource, ClassificationResult, Classifier } from './types.js'

/**
 * Regex-based classifier for PII and secrets.
 */
export class ContentClassifier implements Classifier {
  readonly id = 'universal/content'
  readonly description = 'Detects PII and secrets via Regex'
  readonly outputs = ['pii', 'sensitive_keys']

  private patterns = [
    { key: 'pii', name: 'SSN', regex: /\b\d{3}-\d{2}-\d{4}\b/ },
    { key: 'pii', name: 'Email', regex: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/ },
    { key: 'sensitive_keys', name: 'API Key', regex: /\b(?:key|token|secret)[-_]?(?:id|val)?\s*[:=]\s*["']?[A-Za-z0-9+/=]{20,}["']?/i }
  ]

  async classify(resource: ClassifiableResource): Promise<ClassificationResult> {
    const attributes: Record<string, any> = {}
    const provenance: Record<string, any> = {}

    for (const p of this.patterns) {
      if (p.regex.test(resource.body)) {
        if (p.key === 'pii') {
          attributes.pii = true
        } else {
          attributes[p.key] = true
        }
        
        provenance[p.key] = {
          classifier_id: this.id,
          timestamp: new Date().toISOString(),
          confidence: 0.8,
          context: { matched: p.name }
        }
      }
    }

    return { attributes, provenance }
  }
}
