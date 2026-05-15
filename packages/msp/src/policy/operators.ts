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

export function evaluateOperator(op: Operator, left: any, right: any, data?: any): boolean {
  // Resolve right-side if it's a variable reference
  let resolvedRight = right
  if (typeof right === 'string' && data) {
    if (right.startsWith('subject.') || right.startsWith('resource.') || right.startsWith('context.')) {
      resolvedRight = getValueAtPath(data, right)
    }
  }

  switch (op) {
    case 'eq':
      return left === resolvedRight
    case 'ne':
      return left !== resolvedRight
    case 'in':
      return Array.isArray(resolvedRight) && resolvedRight.includes(left)
    case 'ni':
      return Array.isArray(resolvedRight) && !resolvedRight.includes(left)
    case 'contains':
      return Array.isArray(left) && left.includes(resolvedRight)
    case 'gt':
      return typeof left === typeof resolvedRight && left > resolvedRight
    case 'ge':
      return typeof left === typeof resolvedRight && left >= resolvedRight
    case 'lt':
      return typeof left === typeof resolvedRight && left < resolvedRight
    case 'le':
      return typeof left === typeof resolvedRight && left <= resolvedRight
    case 'exists':
      return left !== undefined && left !== null
    case 'not_exists':
      return left === undefined || left === null
    case 'matches':
      if (typeof left !== 'string' || typeof resolvedRight !== 'string') return false
      try {
        return new RegExp(resolvedRight).test(left)
      } catch {
        return false
      }
    case 'intersect': {
      const leftArr = Array.isArray(left) ? left : left !== undefined && left !== null ? [left] : []
      const rightArr = Array.isArray(resolvedRight)
        ? resolvedRight
        : resolvedRight !== undefined && resolvedRight !== null
          ? [resolvedRight]
          : []
      if (leftArr.length === 0 || rightArr.length === 0) return false
      return leftArr.some((item) => rightArr.includes(item))
    }
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
      if (!evaluateOperator(op as Operator, value, right, data)) {
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
