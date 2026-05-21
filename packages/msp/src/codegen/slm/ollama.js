import { SlmError } from './errors.js';
const DEFAULT_HOST = 'http://127.0.0.1:11434';
const DEFAULT_MODEL = 'qwen2.5-coder:7b';
const DEFAULT_TIMEOUT_MS = 120_000;
const DEFAULT_TEMPERATURE = 0.2;
export function createOllamaClient(opts = {}) {
    const host = (opts.host ?? process.env.OLLAMA_HOST ?? DEFAULT_HOST).replace(/\/+$/, '');
    const model = opts.model ?? process.env.OLLAMA_MODEL ?? DEFAULT_MODEL;
    const fetchImpl = opts.fetchImpl ?? globalThis.fetch;
    const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    const temperature = opts.temperature ?? DEFAULT_TEMPERATURE;
    if (typeof fetchImpl !== 'function') {
        throw new SlmError('global fetch unavailable; pass opts.fetchImpl', 'config');
    }
    return async (call) => {
        const ac = new AbortController();
        const timer = setTimeout(() => ac.abort(), timeoutMs);
        const url = `${host}/api/generate`;
        const body = JSON.stringify({
            model,
            prompt: call.prompt,
            stream: false,
            options: { temperature },
        });
        let resp;
        try {
            resp = await fetchImpl(url, {
                method: 'POST',
                headers: { 'content-type': 'application/json' },
                body,
                signal: ac.signal,
            });
        }
        catch (err) {
            if (ac.signal.aborted) {
                throw new SlmError(`ollama timeout after ${timeoutMs}ms`, 'timeout', err);
            }
            throw new SlmError(`ollama network: ${err.message ?? 'unknown'}`, 'network', err);
        }
        finally {
            clearTimeout(timer);
        }
        if (!resp.ok) {
            const text = await resp.text().catch(() => '');
            throw new SlmError(`ollama http ${resp.status}: ${text.slice(0, 200)}`, 'http');
        }
        let json;
        try {
            json = (await resp.json());
        }
        catch (err) {
            throw new SlmError('ollama: response is not valid JSON', 'parse', err);
        }
        if (typeof json.response !== 'string') {
            throw new SlmError(`ollama: response missing 'response' string (got ${typeof json.response})`, 'parse');
        }
        return json.response;
    };
}
