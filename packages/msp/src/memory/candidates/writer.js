import { mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { assertValidProposedId, composeBody, composeFrontmatter, extractTitle, parseFrontmatter, } from './schema.js';
import { CandidateNotFoundError, } from './types.js';
const DEFAULT_NAMESPACE = 'evaAI';
export class CandidateWriter {
    root;
    namespace;
    proposedBy;
    constructor(opts) {
        this.root = opts.root;
        this.namespace = opts.namespace ?? DEFAULT_NAMESPACE;
        this.proposedBy = opts.proposedBy ?? 'agent';
    }
    candidatesDir() {
        return resolve(this.root, '.brain/msp/projects', this.namespace, 'candidates');
    }
    candidatePath(proposedId) {
        return resolve(this.candidatesDir(), `${proposedId}.md`);
    }
    async write(input) {
        assertValidProposedId(input.proposed_id);
        const path = this.candidatePath(input.proposed_id);
        await mkdir(dirname(path), { recursive: true });
        let overwritten = false;
        try {
            await stat(path);
            overwritten = true;
        }
        catch {
            // ENOENT — fresh write
        }
        const proposedAt = new Date().toISOString();
        const fm = composeFrontmatter(input, proposedAt, this.proposedBy);
        const body = composeBody(input.title, input.body);
        await writeFile(path, `${fm}\n\n${body}\n`, 'utf8');
        return { path, overwritten };
    }
    async list() {
        const dir = this.candidatesDir();
        let entries;
        try {
            entries = await readdir(dir);
        }
        catch (err) {
            if (err.code === 'ENOENT')
                return [];
            throw err;
        }
        const out = [];
        for (const file of entries) {
            if (!file.endsWith('.md'))
                continue;
            const path = resolve(dir, file);
            const raw = await readFile(path, 'utf8');
            const { fm, body } = parseFrontmatter(raw);
            out.push({ ...fm, title: extractTitle(body), path });
        }
        out.sort((a, b) => (b.proposed_at ?? '').localeCompare(a.proposed_at ?? ''));
        return out;
    }
    async read(proposedId) {
        assertValidProposedId(proposedId);
        const path = this.candidatePath(proposedId);
        let raw;
        try {
            raw = await readFile(path, 'utf8');
        }
        catch (err) {
            if (err.code === 'ENOENT') {
                throw new CandidateNotFoundError(proposedId);
            }
            throw err;
        }
        const { fm, body } = parseFrontmatter(raw);
        return { ...fm, title: extractTitle(body), path, body };
    }
    async delete(proposedId) {
        assertValidProposedId(proposedId);
        const path = this.candidatePath(proposedId);
        try {
            await rm(path);
        }
        catch (err) {
            if (err.code === 'ENOENT') {
                throw new CandidateNotFoundError(proposedId);
            }
            throw err;
        }
    }
}
