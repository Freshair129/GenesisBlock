import { describe, expect, it } from 'vitest'
import { runClassifiers } from '../../src/policy/classifiers/engine.js'
import { CodingClassifier } from '../../src/policy/classifiers/coding.js'
import type { ClassifiableResource } from '../../src/policy/classifiers/types.js'

describe('Coding Domain Pack: Classifier', () => {
  const classifiers = [new CodingClassifier()]

  it('detects language and entrypoint correctly', async () => {
    const res: ClassifiableResource = {
      id: 'index.ts',
      path: 'packages/msp/src/index.ts',
      body: "import express from 'express'"
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.language).toBe('ts')
    expect(result.attributes.is_entrypoint).toBe(true)
    expect(result.attributes.is_test).toBe(false)
    expect(result.attributes.framework).toBe('express')
  })

  it('detects test files correctly', async () => {
    const res: ClassifiableResource = {
      id: 'foo.test.ts',
      path: 'packages/msp/test/foo.test.ts',
      body: "import { describe } from 'vitest'"
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.language).toBe('ts')
    expect(result.attributes.is_test).toBe(true)
    expect(result.attributes.is_entrypoint).toBe(false)
  })

  it('detects react framework correctly', async () => {
    const res: ClassifiableResource = {
      id: 'App.tsx',
      path: 'apps/web/src/App.tsx',
      body: "import React from 'react'"
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.language).toBe('ts')
    expect(result.attributes.is_entrypoint).toBe(true)
    expect(result.attributes.framework).toBe('react')
  })

  it('handles non-coding files gracefully', async () => {
    const res: ClassifiableResource = {
      id: 'DATA.json',
      path: 'data/raw.json',
      body: '{}'
    }
    
    const result = await runClassifiers(res, classifiers)
    expect(result.attributes.language).toBe('json')
    expect(result.attributes.is_test).toBe(false)
  })
})
