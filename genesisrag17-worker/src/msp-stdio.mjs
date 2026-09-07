import { createInterface } from 'node:readline';
import { spawn } from 'node:child_process';

/**
 * Small newline-delimited JSON-RPC client for the standalone MSP stdio
 * transport. The worker never imports GKS or opens an MSP database; every
 * pipeline operation crosses this authenticated tool boundary.
 */
export function createMspStdioCaller({ command, args = [], cwd, env = {}, timeoutMs = 30000 } = {}) {
  if (!command || typeof command !== 'string') {
    throw new Error('MSP_COMMAND_REQUIRED');
  }
  if (!Number.isInteger(timeoutMs) || timeoutMs < 100) throw new Error('MSP_TIMEOUT_INVALID');
  let child;
  let lines;
  let nextId = 1;
  let closed = false;
  const pending = new Map();

  const rejectAll = (error) => {
    for (const { reject, timer } of pending.values()) {
      clearTimeout(timer);
      reject(error);
    }
    pending.clear();
  };

  const ensureChild = () => {
    if (child && !closed) return;
    closed = false;
    child = spawn(command, args, {
      cwd,
      env: { ...process.env, ...env },
      shell: false,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    lines = createInterface({ input: child.stdout, crlfDelay: Infinity });
    lines.on('line', (line) => {
      if (!line.trim()) return;
      let message;
      try {
        message = JSON.parse(line);
      } catch (error) {
        rejectAll(new Error(`MSP_INVALID_JSON:${error.message}`));
        return;
      }
      const waiter = pending.get(message.id);
      if (!waiter) return;
      pending.delete(message.id);
      clearTimeout(waiter.timer);
      if (message.error) {
        waiter.reject(new Error(`MSP_RPC_ERROR:${message.error.message ?? JSON.stringify(message.error)}`));
        return;
      }
      const result = message.result;
      if (result?.isError) {
        const text = result?.content?.find((item) => item.type === 'text')?.text;
        waiter.reject(new Error(`MSP_TOOL_ERROR:${text ?? JSON.stringify(result)}`));
        return;
      }
      waiter.resolve(result?.structuredContent ?? result);
    });
    child.stderr.on('data', () => {
      // MSP diagnostics are deliberately not mixed into the JSON-RPC stream.
    });
    child.once('error', (error) => rejectAll(error));
    child.once('exit', (code, signal) => {
      const error = new Error(`MSP_EXIT:${code ?? 'null'}:${signal ?? 'null'}`);
      rejectAll(error);
      child = undefined;
      lines?.close();
      lines = undefined;
    });
  };

  const call = (method, params) => {
    ensureChild();
    const id = nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        if (!pending.has(id)) return;
        pending.delete(id);
        reject(new Error(`MSP_TIMEOUT:${method}`));
        // A timed out line protocol cannot be safely resynchronised if the
        // server later writes the reply; restart the child for the next call.
        if (child && !child.killed) child.kill();
      }, timeoutMs);
      pending.set(id, { resolve, reject, timer });
      try {
        child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
      } catch (error) {
        pending.delete(id);
        clearTimeout(timer);
        reject(error);
      }
    });
  };

  const callTool = (name, toolArgs) => call('tools/call', { name, arguments: toolArgs });

  callTool.close = () => {
    closed = true;
    rejectAll(new Error('MSP_CLIENT_CLOSED'));
    lines?.close();
    if (child && !child.killed) child.kill();
    child = undefined;
  };
  callTool.process = () => child;
  return callTool;
}
