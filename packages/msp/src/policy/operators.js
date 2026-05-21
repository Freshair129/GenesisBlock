export function evaluateOperator(op, left, right, data) {
    // Resolve right-side if it's a variable reference
    let resolvedRight = right;
    if (typeof right === 'string' && data) {
        if (right.startsWith('subject.') || right.startsWith('resource.') || right.startsWith('context.')) {
            resolvedRight = getValueAtPath(data, right);
        }
    }
    switch (op) {
        case 'eq':
            return left === resolvedRight;
        case 'ne':
            return left !== resolvedRight;
        case 'in':
            return Array.isArray(resolvedRight) && resolvedRight.includes(left);
        case 'ni':
            return Array.isArray(resolvedRight) && !resolvedRight.includes(left);
        case 'contains':
            return Array.isArray(left) && left.includes(resolvedRight);
        case 'gt':
            return typeof left === typeof resolvedRight && left > resolvedRight;
        case 'ge':
            return typeof left === typeof resolvedRight && left >= resolvedRight;
        case 'lt':
            return typeof left === typeof resolvedRight && left < resolvedRight;
        case 'le':
            return typeof left === typeof resolvedRight && left <= resolvedRight;
        case 'exists':
            return left !== undefined && left !== null;
        case 'not_exists':
            return left === undefined || left === null;
        case 'matches':
            if (typeof left !== 'string' || typeof resolvedRight !== 'string')
                return false;
            try {
                return new RegExp(resolvedRight).test(left);
            }
            catch {
                return false;
            }
        case 'intersect': {
            const leftArr = Array.isArray(left) ? left : left !== undefined && left !== null ? [left] : [];
            const rightArr = Array.isArray(resolvedRight)
                ? resolvedRight
                : resolvedRight !== undefined && resolvedRight !== null
                    ? [resolvedRight]
                    : [];
            if (leftArr.length === 0 || rightArr.length === 0)
                return false;
            return leftArr.some((item) => rightArr.includes(item));
        }
        case 'time_within': {
            if (!left || typeof left !== 'string')
                return false;
            const leftDate = new Date(left).getTime();
            if (isNaN(leftDate))
                return false;
            const referenceDate = data?.context?.time ? new Date(data.context.time).getTime() : Date.now();
            const diffSeconds = (referenceDate - leftDate) / 1000;
            return diffSeconds >= 0 && diffSeconds <= (Number(resolvedRight) || 0);
        }
        default:
            return false;
    }
}
export function evaluateCondition(condition, data) {
    if ('all_of' in condition && Array.isArray(condition.all_of)) {
        return condition.all_of.every((c) => evaluateCondition(c, data));
    }
    if ('any_of' in condition && Array.isArray(condition.any_of)) {
        return condition.any_of.some((c) => evaluateCondition(c, data));
    }
    if ('none_of' in condition && Array.isArray(condition.none_of)) {
        return !condition.none_of.some((c) => evaluateCondition(c, data));
    }
    // Attribute path match
    for (const [path, ops] of Object.entries(condition)) {
        if (path === 'all_of' || path === 'any_of' || path === 'none_of')
            continue;
        const value = getValueAtPath(data, path);
        for (const [op, right] of Object.entries(ops)) {
            if (!evaluateOperator(op, value, right, data)) {
                return false;
            }
        }
    }
    return true;
}
function getValueAtPath(obj, path) {
    if (!path.includes('.'))
        return obj[path];
    return path.split('.').reduce((prev, curr) => prev?.[curr], obj);
}
