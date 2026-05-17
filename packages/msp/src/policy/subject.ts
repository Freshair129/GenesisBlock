import { makeSubject, type Subject } from './types.js'
import type { Identity } from '../identity/types.js'

/**
 * Hydrate a Subject from an authenticated Identity.
 *
 * Per BLUEPRINT--PHASE-4-USER-ABAC, this maps an authenticated identity
 * (roles, clearance, mfa_status, tenant_ids) into the Subject AttributeBag.
 */
export function hydrateSubject(identity: Identity): Subject {
  const { profile } = identity

  const subject: Subject = makeSubject('user', profile.name || 'anonymous', {
    role: profile.role,
    tier: profile.tier,
    roles: profile.roles || [],
    clearance: profile.clearance ?? 0,
    mfa_status: profile.mfaStatus ?? false,
    tenant_ids: profile.tenantIds || [],
  })

  // Carry forward last step-up data if available (e.g. from persisted session)
  // In Phase 5, this might be stored in the identity's extensions or preferences
  if (identity.preferences?.last_step_up_at) {
    const at = identity.preferences.last_step_up_at as string
    const method = identity.preferences.last_step_up_method as string
    subject.last_step_up_at = at
    subject.last_step_up_method = method
    // Also inject into attributes for policy matching compatibility
    subject.attributes.last_step_up_at = at
    subject.attributes.last_step_up_method = method
  }

  return subject
}
