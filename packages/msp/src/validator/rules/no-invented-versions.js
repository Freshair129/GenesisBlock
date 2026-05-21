const SEMVER_RE = /^(\d+)\.(\d+)\.(\d+)$/;
const TYPES_WITH_VERSION = new Set(['module', 'feat', 'feature', 'protocol', 'mod', 'proto']);
/**
 * Per ADR--ANTI-HALLUCINATION-RULES (rule #3): semver-only; first draft = 0.1.0;
 * bumps must be one of patch/minor/major from the previous version.
 *
 * Implementation note: the validator only sees one atom at a time. We can't
 * cross-check against the previous version of the same atom without history
 * tracking — that's a bi-temporal concern. What we CAN catch:
 *   - non-semver strings (e.g. "1.5.2-beta", "v1.0", "latest")
 *   - skipping major version (refusing 2.0.0 if there's no prior 1.x.x in
 *     the atomic index for the same id)
 *
 * The cross-version skip-detection is a soft warning, not a hard error,
 * because the validator can't see git history of the atom file.
 */
export function noInventedVersions(atom, ctx) {
    const type = atom.fm.type;
    if (typeof type !== 'string' || !TYPES_WITH_VERSION.has(type.toLowerCase()))
        return [];
    const version = atom.fm.version;
    if (version === undefined || version === null) {
        // Missing version on a type that requires it — caught by required-fields rule (M5d), not here.
        return [];
    }
    if (typeof version !== 'string') {
        return [
            {
                rule: 'no-invented-versions',
                severity: 'error',
                message: `version must be a string, got ${typeof version}`,
                offending: String(version),
            },
        ];
    }
    if (!SEMVER_RE.test(version)) {
        return [
            {
                rule: 'no-invented-versions',
                severity: 'error',
                message: `version '${version}' is not semver (expected MAJOR.MINOR.PATCH)`,
                offending: version,
            },
        ];
    }
    // First-draft check: if id is in the index already, presumably we're updating.
    // If the new major is > 1 and the id is NEW (not in index), that's suspicious
    // — first draft should be 0.1.0 per spec.
    const id = atom.fm.id ?? atom.fm.proposed_id;
    const existing = typeof id === 'string' ? ctx.atomicIndex.get(id) : undefined;
    if (!existing) {
        const [maj] = version.split('.').map((p) => Number.parseInt(p, 10));
        if (maj !== undefined && maj > 0) {
            return [
                {
                    rule: 'no-invented-versions',
                    severity: 'error',
                    message: `new atom '${id ?? '<no id>'}' starts at version '${version}' — first draft must be 0.1.0`,
                    offending: version,
                },
            ];
        }
    }
    return [];
}
