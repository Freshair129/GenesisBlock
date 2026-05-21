import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { readIdentity } from '../identity/store.js';
import { hydrateSubject } from '../policy/subject.js';
import { makeContext } from '../policy/types.js';
import * as backlinksRebuild from './tools/backlinks-rebuild.js';
import * as brainResolve from './tools/brain-resolve.js';
import * as candidate from './tools/candidate.js';
import * as compressTool from './tools/compress.js';
import * as dispatchTool from './tools/dispatch.js';
import * as distillTool from './tools/distill.js';
import * as identityBeliefsTool from './tools/identity-beliefs.js';
import * as episodeAppend from './tools/episode-append.js';
import * as identityGet from './tools/identity-get.js';
import * as identitySet from './tools/identity-set.js';
import * as projectList from './tools/project-list.js';
import * as projectRegister from './tools/project-register.js';
import * as projectResolve from './tools/project-resolve.js';
import * as recallTool from './tools/recall.js';
import * as remember from './tools/remember.js';
import * as runTask from './tools/run-task.js';
import * as sessionAppend from './tools/session-append.js';
import * as symbolCommunity from './tools/symbol-community.js';
import * as symbolImpact from './tools/symbol-impact.js';
import * as symbolLookup from './tools/symbol-lookup.js';
import * as symbolNeighbors from './tools/symbol-neighbors.js';
import * as symbolSearch from './tools/symbol-search.js';
import * as symbolTrace from './tools/symbol-trace.js';
import * as validateTool from './tools/validate.js';
import * as escalateTool from './tools/escalate.js';
import * as expandTool from './tools/expand.js';
const TOOLS = [
    validateTool,
    brainResolve,
    dispatchTool,
    distillTool,
    identityBeliefsTool,
    candidate,
    runTask,
    sessionAppend,
    episodeAppend,
    backlinksRebuild,
    recallTool,
    remember,
    compressTool,
    identityGet,
    identitySet,
    projectList,
    projectRegister,
    projectResolve,
    symbolLookup,
    symbolNeighbors,
    symbolImpact,
    symbolCommunity,
    symbolSearch,
    symbolTrace,
    escalateTool,
    expandTool,
];
export async function createMspMcpServer(opts = {}) {
    const root = opts.root ?? process.env.MSP_ROOT ?? process.cwd();
    // Resolve identity once for the server (assuming stdio/local user)
    const identity = await readIdentity({ root });
    const subject = hydrateSubject(identity);
    const policyContext = makeContext('mcp-stdio', `mcp-${Date.now()}`);
    const ctx = { root, subject, policyContext };
    const server = new McpServer({
        name: 'msp',
        version: '0.1.0',
    });
    for (const t of TOOLS) {
        server.registerTool(t.name, {
            description: t.description,
            inputSchema: t.inputSchema,
        }, 
        // SDK accepts our handler shape: (args) => Promise<{content,isError?}>
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        t.handler(ctx));
    }
    return server;
}
/** Names of all tools registered by createMspMcpServer (for tests). */
export const REGISTERED_TOOL_NAMES = TOOLS.map((t) => t.name);
