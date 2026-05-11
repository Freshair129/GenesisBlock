/**
 * PROTO--SUMMARY-MIN — minimum/maximum summary length + placeholder ban.
 * Wraps the existing `summaryMin` core rule via `ruleAdapter`.
 *
 * Ships draft (M8f promotion). The original rule continues to run as a
 * structural rule until this PROTO is promoted to stable.
 */

import { summaryMin } from '../rules/summary-min.js'
import { ruleAdapter } from './rule-adapter.js'

export default ruleAdapter(summaryMin)
