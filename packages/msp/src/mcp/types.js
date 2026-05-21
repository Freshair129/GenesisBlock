/** Convenience: wrap a value as a single-text MCP tool result. */
export function jsonResult(value) {
    return { content: [{ type: 'text', text: JSON.stringify(value, null, 2) }] };
}
export function errorResult(message) {
    return { content: [{ type: 'text', text: message }], isError: true };
}
