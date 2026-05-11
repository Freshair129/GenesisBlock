/**
 * PROTO--ADR-MONOTONIC — ADR-NNN must equal max(existing) + 1.
 * Wraps the existing `adrMonotonic` core rule via `ruleAdapter`.
 *
 * Ships draft (M8f promotion).
 */

import { adrMonotonic } from '../rules/adr-monotonic.js'
import { ruleAdapter } from './rule-adapter.js'

export default ruleAdapter(adrMonotonic, {
  filter: (entry) => entry.type === 'adr',
})
