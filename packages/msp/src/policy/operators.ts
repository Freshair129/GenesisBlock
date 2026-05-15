import type { JsonValue } from './types.js'

/**
 * Minimal operator set for YAML policy conditions.
 * See ADR--POLICY-AS-DATA-NOT-CODE.
 */
export type Operator =
  | 'eq' // equal
  | 'ne' // not equal
  | 'in' // member of (right must be array)
  | 'ni' // not member of (right must be array)
  | 'contains' // contains (left must be array)
  | 'gt' // greater than
  | 'ge' // greater than or equal
  | 'lt' // less than
  | 'le' // less than or equal
  | 'exists' // left is not undefined/null (right ignored)
  | 'not_exists' // left is undefined/null (right ignored)
  | 'matches' // regex match (right must be string)
  | 'intersect' // set intersection non-empty (both must be arrays)

export function evaluateOperator(op: Operator, left: any, right: any): boolean {
  switch (op) {
    case 'eq':
      return left === right
    case 'ne':
      return left !== right
    case 'in':
      return Array.isArray(right) && right.includes(left)
    case 'ni':
      return Array.isArray(right) && !right.includes(left)
    case 'contains':
      return Array.isArray(left) && left.includes(right)
    case 'gt':
      return typeof left === typeof right && left > right
    case 'ge':
      return typeof left === typeof right && left >= right
    case 'lt':
      return typeof left === typeof right && left < right
    case 'le':
      return typeof left === typeof right && left <= right
    case 'exists':
      return left !== undefined && left !== null
    case 'not_exists':
      return left === undefined || left === null
    case 'matches':
      if (typeof left !== 'string' || typeof right !== 'string') return false
      try {
        return new RegExp(right).test(left)
      } catch {
        return false
      }
    case 'intersect':
      if (!Array.isArray(left) || !Array.isArray(right)) return false
      return left.some((item) => right.includes(item))
    default:
      return false
  }
}

/**
 * Evaluates a condition object.
 * Format: { [attribute_path]: { [operator]: value } }
 * or logical group: { all_of: [condition...] }
 */
export type Condition =
  | { all_of: Condition[] }
  | { any_of: Condition[] }
  | { none_of: Condition[] }
  | { [path: string]: { [op in Operator]?: any } }

export function evaluateCondition(condition: Condition, data: Record<string, any>): boolean {
  if ('all_of' in condition && Array.isArray(condition.all_of)) {
    return (condition.all_of as Condition[]).every((c) => evaluateCondition(c, data))
  }
  if ('any_of' in condition && Array.isArray(condition.any_of)) {
    return (condition.any_of as Condition[]).some((c) => evaluateCondition(c, data))
  }
  if ('none_of' in condition && Array.isArray(condition.none_of)) {
    return !(condition.none_of as Condition[]).some((c) => evaluateCondition(c, data))
  }

  // Attribute path match
  for (const [path, ops] of Object.entries(condition)) {
    if (path === 'all_of' || path === 'any_of' || path === 'none_of') continue
    const value = getValueAtPath(data, path)
    for (const [op, right] of Object.entries(ops as Record<Operator, any>)) {
      if (!evaluateOperator(op as Operator, value, right)) {
        return false
      }
    }
  }

  return true
}

function getValueAtPath(obj: any, path: string): any {
  if (!path.includes('.')) return obj[path]
  return path.split('.').reduce((prev, curr) => prev?.[curr], obj)
}
