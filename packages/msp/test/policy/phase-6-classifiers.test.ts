import { describe, expect, it } from 'vitest'
import { runClassifiers } from '../../src/policy/classifiers/engine.js'
import { PathClassifier } from '../../src/policy/classifiers/path.js'
import { ContentClassifier } from '../../src/policy/classifiers/content.js'
import type { ClassifiableResource } from '../../src/policy/classifiers/types.js'

describe('UCF Phase 6: Classifiers', () => {
  const classifiers = [new PathClassifier(), new ContentClassifier()]

  it('tags domain correctly from path', async () => {
    const res: ClassifiableResource = {
      id: 'ADR--1',
      path: 'gks/adr/ADR--1.md',
      body: '# Title\nContent'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.domain).toBe('adr')
    expect(result.provenance.domain.classifier_id).toBe('universal/path')
  })

  it('tags pii correctly from content', async () => {
    const res: ClassifiableResource = {
      id: 'FACT--1',
      path: 'gks/fact/FACT--1.md',
      body: 'Patient SSN is 123-45-6789'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.pii).toBe(true)
    expect(result.provenance.pii.classifier_id).toBe('universal/content')
  })

  it('preserves manual frontmatter over auto-tags', async () => {
    const res: ClassifiableResource = {
      id: 'ADR--1',
      path: 'gks/adr/ADR--1.md',
      body: '# Title',
      attributes: { domain: 'overridden' }
    }
    
    const result = await runClassifiers(res, classifiers)
    // PathClassifier would want 'adr', but manual frontmatter 'overridden' wins
    expect(result.attributes.domain).toBe('overridden')
    expect(result.provenance.domain.classifier_id).toBe('manual/frontmatter')
  })

  it('detects multiple attributes from multiple classifiers', async () => {
    const res: ClassifiableResource = {
      id: 'FEAT--1',
      path: 'gks/feat/FEAT--1.md',
      body: 'Contains email: alice@example.com'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.domain).toBe('feat')
    expect(result.attributes.pii).toBe(true)
  })
})
