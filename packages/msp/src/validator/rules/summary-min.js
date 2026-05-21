const FORBIDDEN_PLACEHOLDERS = ['TBD', 'TODO', 'FIXME', 'lorem ipsum'];
const MIN_LEN = 10;
const MAX_LEN = 300;
export function summaryMin(atom, _ctx) {
    const summary = atom.fm['summary'];
    if (summary === undefined || summary === null)
        return [];
    if (typeof summary !== 'string') {
        return [
            {
                rule: 'summary-min',
                severity: 'error',
                message: 'summary must be a string',
                offending: String(summary),
            },
        ];
    }
    const errors = [];
    const trimmed = summary.trim();
    if (trimmed.length < MIN_LEN) {
        errors.push({
            rule: 'summary-min',
            severity: 'error',
            message: `summary must be at least ${MIN_LEN} characters (got ${trimmed.length})`,
            offending: summary,
        });
    }
    if (trimmed.length > MAX_LEN) {
        errors.push({
            rule: 'summary-min',
            severity: 'error',
            message: `summary must be at most ${MAX_LEN} characters (got ${trimmed.length})`,
            offending: summary,
        });
    }
    for (const placeholder of FORBIDDEN_PLACEHOLDERS) {
        if (trimmed.toLowerCase().includes(placeholder.toLowerCase())) {
            errors.push({
                rule: 'summary-min',
                severity: 'error',
                message: `summary contains placeholder '${placeholder}' — write a real summary`,
                offending: placeholder,
            });
            break;
        }
    }
    return errors;
}
