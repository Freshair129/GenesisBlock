let warnedHost = false;
export function _resetWarnedForTests() {
    warnedHost = false;
}
export function resolveEnv(override, warn) {
    const env = process.env;
    let url = override.url ?? env.OBSIDIAN_URL;
    const hostDeprecated = !env.OBSIDIAN_URL && !override.url && !!env.OBSIDIAN_HOST;
    if (hostDeprecated) {
        url = env.OBSIDIAN_HOST;
        if (!warnedHost) {
            warnedHost = true;
            warn('[msp/obsidian] OBSIDIAN_HOST is deprecated; use OBSIDIAN_URL instead.');
        }
    }
    return {
        url: url || undefined,
        apiKey: override.apiKey ?? env.OBSIDIAN_API_KEY,
        insecure: env.OBSIDIAN_INSECURE === 'true',
        hostDeprecated,
    };
}
export function isLoopback(url) {
    try {
        const u = new URL(url);
        const h = u.hostname.replace(/^\[|\]$/g, '');
        return h === '127.0.0.1' || h === 'localhost' || h === '::1';
    }
    catch {
        return false;
    }
}
