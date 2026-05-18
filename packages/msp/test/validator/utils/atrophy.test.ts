import { describe, expect, it } from 'vitest'
import {
  AtrophyStatus,
  getAtrophyStatus,
  parseValidUntil,
} from '../../../src/validator/utils/atrophy.js'

describe('Atrophy Utils', () => {
  describe('parseValidUntil', () => {
    it('should return string as is', () => {
      expect(parseValidUntil('2026-01-01')).toBe('2026-01-01')
    })

    it('should convert Date object to ISO string', () => {
      const d = new Date('2026-01-01T10:00:00Z')
      expect(parseValidUntil(d)).toBe(d.toISOString())
    })

    it('should return null for invalid types', () => {
      expect(parseValidUntil(null)).toBeNull()
      expect(parseValidUntil(undefined)).toBeNull()
      expect(parseValidUntil(123)).toBeNull()
      expect(parseValidUntil({})).toBeNull()
    })

    it('should return null for invalid Date', () => {
      expect(parseValidUntil(new Date('invalid'))).toBeNull()
    })
  })

  describe('getAtrophyStatus', () => {
    const now = new Date('2026-05-15T12:00:00Z')

    it('should return EXPIRED for past date', () => {
      const result = getAtrophyStatus('2026-05-14T00:00:00Z', now)
      expect(result?.status).toBe(AtrophyStatus.EXPIRED)
      expect(result?.daysUntilExpiry).toBeLessThan(0)
    })

    it('should return NEAR_EXPIRY for date within 30 days', () => {
      const result = getAtrophyStatus('2026-06-10T12:00:00Z', now)
      expect(result?.status).toBe(AtrophyStatus.NEAR_EXPIRY)
      expect(result?.daysUntilExpiry).toBeGreaterThanOrEqual(0)
      expect(result?.daysUntilExpiry).toBeLessThan(30)
    })

    it('should return HEALTHY for future date > 30 days', () => {
      const result = getAtrophyStatus('2026-07-01T12:00:00Z', now)
      expect(result?.status).toBe(AtrophyStatus.HEALTHY)
      expect(result?.daysUntilExpiry).toBeGreaterThanOrEqual(30)
    })

    it('should support custom threshold', () => {
      const result = getAtrophyStatus('2026-06-01T12:00:00Z', now, 10)
      // Jun 1 is 17 days from May 15. Threshold is 10.
      expect(result?.status).toBe(AtrophyStatus.HEALTHY)
    })

    it('should return null for invalid date string', () => {
      expect(getAtrophyStatus('not-a-date', now)).toBeNull()
    })
  })
})
