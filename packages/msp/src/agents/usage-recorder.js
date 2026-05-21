import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
/**
 * Best-effort daily-bucket usage recorder for dispatcher cost telemetry.
 *
 * Appends each dispatch's (tier, cost_usd) to today's USAGE--DAILY-<isoDate>
 * atom under <root>/gks/usage/. On first call of the day, creates the atom
 * with proper frontmatter (per SPEC--USAGE-ATOM). On subsequent calls, parses
 * the JSON summary block between the markers and updates totals in place.
 *
 * Failures (mkdir/read/parse/write errors) propagate — dispatch.ts wraps the
 * call in try/catch and treats this as best-effort, matching recordEpisode.
 */
const SUMMARY_START = '<!-- USAGE-SUMMARY-START -->';
const SUMMARY_END = '<!-- USAGE-SUMMARY-END -->';
const TOP_EPISODES_LIMIT = 5;
function emptySummary(now) {
    return {
        total_cost_usd: 0,
        call_count: 0,
        by_tier: {
            T1: { count: 0, cost_usd: 0 },
            T2: { count: 0, cost_usd: 0 },
            T3: { count: 0, cost_usd: 0 },
        },
        top_episodes: [],
        updated_at: now,
    };
}
function applyEpisode(summary, input, now) {
    summary.total_cost_usd += input.cost_usd;
    summary.call_count += 1;
    const bucket = summary.by_tier[input.tier];
    bucket.count += 1;
    bucket.cost_usd += input.cost_usd;
    if (input.episode_id !== undefined && input.cost_usd > 0) {
        summary.top_episodes.push({
            id: input.episode_id,
            cost_usd: input.cost_usd,
            tier: input.tier,
        });
        summary.top_episodes.sort((a, b) => b.cost_usd - a.cost_usd);
        if (summary.top_episodes.length > TOP_EPISODES_LIMIT) {
            summary.top_episodes.length = TOP_EPISODES_LIMIT;
        }
    }
    summary.updated_at = now;
    return summary;
}
function buildFrontmatter(id, title, createdAt) {
    return [
        '---',
        `id: ${id}`,
        'phase: 5',
        'type: usage',
        'status: stable',
        'tier: genesis',
        'source_type: episodic',
        'vault_id: default',
        `title: ${escapeYaml(title)}`,
        'tags:',
        '  - agents',
        '  - usage',
        '  - cost',
        '  - daily',
        `created_at: ${createdAt}`,
        '---',
        '',
    ].join('\n');
}
function buildBody(title, summary) {
    const json = JSON.stringify(summary, null, 2);
    return [
        `# ${title}`,
        '',
        'Daily aggregate of dispatcher cost telemetry. See `SPEC--USAGE-ATOM` for the contract.',
        '',
        '## Summary',
        '',
        SUMMARY_START,
        '```json',
        json,
        '```',
        SUMMARY_END,
        '',
    ].join('\n');
}
function escapeYaml(s) {
    return `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}
function isoDate(now) {
    return now.toISOString().slice(0, 10);
}
async function fileExists(path) {
    try {
        await readFile(path, 'utf8');
        return true;
    }
    catch (err) {
        if (typeof err === 'object' &&
            err !== null &&
            'code' in err &&
            err.code === 'ENOENT') {
            return false;
        }
        throw err;
    }
}
function extractSummary(content) {
    const startIdx = content.indexOf(SUMMARY_START);
    const endIdx = content.indexOf(SUMMARY_END);
    if (startIdx === -1 || endIdx === -1 || endIdx < startIdx)
        return null;
    const block = content.slice(startIdx + SUMMARY_START.length, endIdx);
    const fenceMatch = block.match(/```json\s*([\s\S]*?)\s*```/);
    if (fenceMatch === null)
        return null;
    try {
        const parsed = JSON.parse(fenceMatch[1]);
        // Defensive: ensure tier buckets exist even if file was hand-edited.
        if (!parsed.by_tier)
            parsed.by_tier = emptySummary('').by_tier;
        for (const t of ['T1', 'T2', 'T3']) {
            if (!parsed.by_tier[t])
                parsed.by_tier[t] = { count: 0, cost_usd: 0 };
        }
        if (!Array.isArray(parsed.top_episodes))
            parsed.top_episodes = [];
        return parsed;
    }
    catch {
        return null;
    }
}
function replaceSummaryBlock(content, summary) {
    const startIdx = content.indexOf(SUMMARY_START);
    const endIdx = content.indexOf(SUMMARY_END);
    if (startIdx === -1 || endIdx === -1) {
        // Shouldn't happen if extractSummary returned non-null; fall back to append.
        return content + '\n' + buildBody('', summary);
    }
    const before = content.slice(0, startIdx + SUMMARY_START.length);
    const after = content.slice(endIdx);
    const json = JSON.stringify(summary, null, 2);
    return `${before}\n\`\`\`json\n${json}\n\`\`\`\n${after}`;
}
export async function recordUsage(input, root) {
    const now = new Date();
    const nowIso = now.toISOString();
    const dateStr = isoDate(now);
    const id = `USAGE--DAILY-${dateStr}`;
    const title = `USAGE — Daily cost bucket ${dateStr}`;
    const filename = `${id}.md`;
    const absDir = resolve(root, 'gks', 'usage');
    const absPath = join(absDir, filename);
    await mkdir(dirname(absPath), { recursive: true });
    const exists = await fileExists(absPath);
    if (!exists) {
        const summary = applyEpisode(emptySummary(nowIso), input, nowIso);
        const frontmatter = buildFrontmatter(id, title, nowIso);
        const body = buildBody(title, summary);
        await writeFile(absPath, frontmatter + body, 'utf8');
        return absPath;
    }
    const existing = await readFile(absPath, 'utf8');
    const parsed = extractSummary(existing) ?? emptySummary(nowIso);
    const updated = applyEpisode(parsed, input, nowIso);
    const next = replaceSummaryBlock(existing, updated);
    await writeFile(absPath, next, 'utf8');
    return absPath;
}
