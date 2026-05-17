import type { AttributeBag } from '../types.js'

/**
 * Metadata about where an attribute came from.
 * Per BLUEPRINT--PHASE-6-CLASSIFIERS.
 */
export interface Provenance {
  classifier_id: string
  version?: string
  confidence?: number
  timestamp: string
}

/**
 * A Resource as seen by a classifier.
 * Includes minimum fields needed for detection.
 */
export interface ClassifiableResource {
  id: string
  path: string
  body: string
  /** Existing attributes (e.g. from frontmatter). */
  attributes?: AttributeBag
}

/**
 * Result of a single classifier run.
 */
export interface ClassificationResult {
  /** The derived attributes. */
  attributes: AttributeBag
  /** Provenance metadata for each attribute (key matches attributes). */
  provenance: Record<string, Provenance>
}

/**
 * Common interface for all Classifier plugins.
 * Implements FEAT--CLASSIFIER-PLUGINS.
 */
export interface Classifier {
  /** Unique ID of the classifier (e.g. 'universal/path'). */
  id: string
  /** Human-readable description. */
  description: string
  /** Attribute names this classifier is capable of producing. */
  outputs: string[]
  /**
   * Determine attributes for a resource.
   */
  classify(resource: ClassifiableResource): Promise<ClassificationResult>
}
