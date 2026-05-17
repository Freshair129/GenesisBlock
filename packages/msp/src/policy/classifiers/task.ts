import { parse as parseYaml } from 'yaml'
import type { ClassifiableResource, ClassificationResult, Classifier } from './types.js'

/**
 * specialized classifier for Task & Management (Ops) domain.
 * target atoms: ISSUE, RUNBOOK, INCIDENT.
 * extracts: priority, status, assignee from frontmatter.
 *
 * Implements FEAT--TASK-MANAGEMENT-PACK.
 */
export class TaskClassifier implements Classifier {
  readonly id = 'domain/task'
  readonly description = 'Operational management classifier (priority, status, assignee)'
  readonly outputs = ['issue_priority', 'issue_status', 'assignee', 'is_operational']

  private targetPrefixes = new Set(['ISSUE', 'RUNBOOK', 'INCIDENT', 'INC'])

  async classify(resource: ClassifiableResource): Promise<ClassificationResult> {
    const attributes: Record<string, any> = {}
    const provenance: Record<string, any> = {}
    const timestamp = new Date().toISOString()

    const prefix = resource.id.split('--')[0]
    if (!prefix || !this.targetPrefixes.has(prefix)) {
      return { attributes, provenance }
    }

    attributes.is_operational = true
    provenance.is_operational = { classifier_id: this.id, timestamp, confidence: 1.0 }

    // Always try to parse frontmatter from the body to find top-level fields 
    // like 'priority' and 'status', even if some attributes were already passed.
    let fullFm: Record<string, any> = { ...(resource.attributes || {}) }
    
    const fmMatch = resource.body.match(/^---\r?\n([\s\S]*?)\r?\n---/)
    if (fmMatch) {
      try {
        const parsed = parseYaml(fmMatch[1])
        fullFm = { ...parsed, ...fullFm }
      } catch {
        // ignore parse errors
      }
    }

    // Extract specific fields from the combined frontmatter
    if (fullFm.priority) {
      attributes.issue_priority = String(fullFm.priority).toLowerCase()
      provenance.issue_priority = { classifier_id: this.id, timestamp, confidence: 1.0 }
    }
    if (fullFm.status) {
      attributes.issue_status = String(fullFm.status).toLowerCase()
      provenance.issue_status = { classifier_id: this.id, timestamp, confidence: 1.0 }
    }
    if (fullFm.assignee) {
      attributes.assignee = String(fullFm.assignee)
      provenance.assignee = { classifier_id: this.id, timestamp, confidence: 1.0 }
    }

    return { attributes, provenance }
  }
}
