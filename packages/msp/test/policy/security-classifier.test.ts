import { describe, expect, it } from 'vitest'
import { runClassifiers } from '../../src/policy/classifiers/engine.js'
import { SecurityClassifier } from '../../src/policy/classifiers/security.js'
import type { ClassifiableResource } from '../../src/policy/classifiers/types.js'

describe('Security Domain Pack: Classifier', () => {
  const classifiers = [new SecurityClassifier()]

  it('detects AWS keys correctly', async () => {
    const res: ClassifiableResource = {
      id: 'AWS_CREDENTIALS',
      path: 'config/aws.yaml',
      body: 'access_key: AKIA1234567890ABCDEF'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.has_secret).toBe(true)
    expect(result.attributes.secret_type).toBe('aws_key')
    expect(result.attributes.leak_risk).toBe('high')
  })

  it('detects OpenAI keys correctly', async () => {
    const res: ClassifiableResource = {
      id: 'OPENAI_KEY',
      path: '.env',
      body: 'OPENAI_API_KEY=sk-1234567890abcdef1234567890abcdef1234567890abcdef'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.has_secret).toBe(true)
    expect(result.attributes.secret_type).toBe('openai')
  })

  it('detects high-entropy random strings', async () => {
    const res: ClassifiableResource = {
      id: 'RANDOM_BLOB',
      path: 'data/blob.txt',
      body: 'Some random string: 4f7G9xZ2kLp6mN8vQ1rW3tY5uI0oP9aS8dF7gH6jJ5kK4l'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.has_secret).toBe(true)
    expect(result.attributes.secret_type).toBe('high_entropy_string')
  })

  it('detects vaulted content', async () => {
    const res: ClassifiableResource = {
      id: 'VAULTED_DATA',
      path: 'gks/secret/S1.md',
      body: 'content: vault: v1:some-encrypted-data'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.encryption_level).toBe('vault')
  })

  it('handles clean files correctly', async () => {
    const res: ClassifiableResource = {
      id: 'README',
      path: 'README.md',
      body: '# Welcome\nNo secrets here.'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.has_secret).toBe(false)
    expect(result.attributes.leak_risk).toBe('low')
  })
})
