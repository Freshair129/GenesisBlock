import { createOllamaClient } from './ollama.js';
import { createQwenClient } from './qwen.js';
import { createGeminiClient } from './gemini.js';
import { SlmError } from './errors.js';
const mockClient = async ({ prompt }) => {
    const m = prompt.match(/MOCK_OUTPUT:\s*([\s\S]+?)(?:\n##|$)/);
    if (m)
        return m[1].trim() + '\n';
    return 'export const handler = async () => ({ ok: true })\n';
};
export function createSlmClient(opts = {}) {
    const provider = opts.provider ??
        process.env.MSP_SLM_PROVIDER ??
        'ollama';
    switch (provider) {
        case 'ollama':
            return createOllamaClient(opts.ollama ?? {});
        case 'qwen':
            return createQwenClient(opts.qwen ?? {});
        case 'gemini':
            return createGeminiClient(opts.gemini ?? {});
        case 'mock':
            return mockClient;
        default:
            throw new SlmError(`unknown SLM provider '${provider}'`, 'config');
    }
}
export { createOllamaClient } from './ollama.js';
export { createGeminiClient, runGeminiCli } from './gemini.js';
export { SlmError } from './errors.js';
