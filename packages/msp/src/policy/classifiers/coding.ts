import { extname, basename } from 'node:path'
import type { ClassifiableResource, ClassificationResult, Classifier } from './types.js'

/**
 * Specialized classifier for Software Engineering (Coding) domain.
 * Detects programming language, tests, and entry points.
 *
 * Implements FEAT--CODING-DOMAIN-PACK.
 */
export class CodingClassifier implements Classifier {
  readonly id = 'domain/coding'
  readonly description = 'Software engineering classifier (language, tests, entrypoints)'
  readonly outputs = ['language', 'is_test', 'is_entrypoint', 'framework']

  private extensionMap: Record<string, string> = {
    '.ts': 'ts',
    '.tsx': 'ts',
    '.js': 'js',
    '.jsx': 'js',
    '.rs': 'rs',
    '.py': 'py',
    '.go': 'go',
    '.md': 'markdown',
    '.yaml': 'yaml',
    '.yml': 'yaml',
    '.json': 'json',
  }

  private entrypointFiles = new Set(['index.ts', 'main.ts', 'App.tsx', 'bin.ts', 'server.ts', 'cli.ts'])

  async classify(resource: ClassifiableResource): Promise<ClassificationResult> {
    const attributes: Record<string, any> = {}
    const provenance: Record<string, any> = {}
    const timestamp = new Date().toISOString()

    const ext = extname(resource.path).toLowerCase()
    const filename = basename(resource.path)

    // 1. Language detection
    if (this.extensionMap[ext]) {
      attributes.language = this.extensionMap[ext]
      provenance.language = { classifier_id: this.id, timestamp, confidence: 1.0 }
    }

    // 2. Test detection
    if (
      filename.includes('.test.') ||
      filename.includes('.spec.') ||
      resource.path.includes('/test/') ||
      resource.path.includes('/tests/')
    ) {
      attributes.is_test = true
      provenance.is_test = { classifier_id: this.id, timestamp, confidence: 1.0 }
    } else {
      attributes.is_test = false
      provenance.is_test = { classifier_id: this.id, timestamp, confidence: 1.0 }
    }

    // 3. Entrypoint detection
    if (this.entrypointFiles.has(filename)) {
      attributes.is_entrypoint = true
      provenance.is_entrypoint = { classifier_id: this.id, timestamp, confidence: 0.9 }
    } else {
      attributes.is_entrypoint = false
      provenance.is_entrypoint = { classifier_id: this.id, timestamp, confidence: 0.9 }
    }

    // 4. Framework detection (Simple content markers)
    if (resource.body.includes('import React') || resource.body.includes("from 'react'")) {
      attributes.framework = 'react'
      provenance.framework = { classifier_id: this.id, timestamp, confidence: 0.8 }
    } else if (resource.body.includes("from 'express'") || resource.body.includes('import express')) {
      attributes.framework = 'express'
      provenance.framework = { classifier_id: this.id, timestamp, confidence: 0.8 }
    }

    return { attributes, provenance }
  }
}
