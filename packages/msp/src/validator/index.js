import { readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { parseFile } from './parse.js';
import { adrMonotonic } from './rules/adr-monotonic.js';
import { citeOrMarkInferred } from './rules/cite-or-mark-inferred.js';
import { danglingWikilinks } from './rules/dangling-wikilinks.js';
import { evidenceForDecisions } from './rules/evidence-for-decisions.js';
import { forbiddenFields } from './rules/forbidden-fields.js';
import { futureDate } from './rules/future-date.js';
import { idFilenameMatch } from './rules/id-filename-match.js';
import { idFormat } from './rules/id-format.js';
import { masterRequiresPromotion } from './rules/master-requires-promotion.js';
import { noInventedVersions } from './rules/no-invented-versions.js';
import { phaseStatus } from './rules/phase-status.js';
import { registryDrift } from './rules/registry-drift.js';
import { requiredFields } from './rules/required-fields.js';
import { summaryMin } from './rules/summary-min.js';
import { tierEnum } from './rules/tier-enum.js';
import { ValidatorIOError, } from './types.js';
export const HARD_RULES = [
    requiredFields,
    forbiddenFields,
    idFormat,
    idFilenameMatch,
    adrMonotonic,
    danglingWikilinks,
    futureDate,
    summaryMin,
    phaseStatus,
    registryDrift,
    noInventedVersions,
    evidenceForDecisions,
    masterRequiresPromotion,
];
// Soft rules (warnings only; don't fail exit code)
export const SOFT_RULES = [citeOrMarkInferred, tierEnum];
const MAX_ERRORS_PER_FILE = 50;
export async function validate(filepath, ctx) {
    const result = { filepath, errors: [], warnings: [] };
    let atom;
    try {
        atom = await parseFile(filepath);
    }
    catch (err) {
        if (err instanceof ValidatorIOError) {
            result.errors.push({
                rule: 'parse',
                severity: 'error',
                message: err.message,
            });
            return result;
        }
        throw err;
    }
    for (const rule of HARD_RULES) {
        if (result.errors.length >= MAX_ERRORS_PER_FILE)
            break;
        for (const e of rule(atom, ctx)) {
            if (e.severity === 'error') {
                if (result.errors.length < MAX_ERRORS_PER_FILE)
                    result.errors.push(e);
            }
            else {
                result.warnings.push(e);
            }
        }
    }
    for (const rule of SOFT_RULES) {
        for (const e of rule(atom, ctx)) {
            // Soft rules emit only warnings even if rule output marks 'error'.
            result.warnings.push({ ...e, severity: 'warning' });
        }
    }
    return result;
}
async function* walkMd(dir) {
    let entries;
    try {
        entries = await readdir(dir, { withFileTypes: true });
    }
    catch (err) {
        const code = err.code;
        if (code === 'ENOENT')
            return;
        throw err;
    }
    for (const entry of entries) {
        const p = join(dir, entry.name);
        if (entry.isDirectory()) {
            if (entry.name === '00_index')
                continue;
            if (entry.name === 'candidates')
                continue; // proposed atoms; not yet promoted
            yield* walkMd(p);
        }
        else if (entry.isFile() && entry.name.endsWith('.md')) {
            yield p;
        }
    }
}
export async function validateAll(rootDirs, ctx) {
    const out = [];
    for (const dir of rootDirs) {
        for await (const file of walkMd(dir)) {
            out.push(await validate(file, ctx));
        }
    }
    return out;
}
export { loadAtomicIndex } from './atomic-index.js';
