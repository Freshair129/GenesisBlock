/**
 * PROTO--EVIDENCE-FOR-DECISIONS — ADR body requires Context/Decision/Consequences headings.
 * Wraps the existing `evidenceForDecisions` core rule via `ruleAdapter`.
 *
 * Ships draft (M8f promotion).
 */

import { evidenceForDecisions } from '../rules/evidence-for-decisions.js'
import { ruleAdapter } from './rule-adapter.js'

export default ruleAdapter(evidenceForDecisions, {
  filter: (entry) => entry.type === 'adr',
})
