/**
 * Cognitive Layer types — public surface for `createCognitiveLayer`.
 *
 * The facade unifies the GKS storage primitives, the MSP passport
 * (identity / orchestration), and the codegen runner behind one entry
 * point so consumers (EVA, Claude Code, Hermes, openclaw, Cursor, custom
 * MCP agents) wire one factory call instead of stitching MemoryStore,
 * identity, mcp-server, and runner manually.
 *
 * The shape honours seven points from FRAMEWORK_MASTER_SPEC.md (see the
 * inline §-references below).
 */
export class ScaleLevelGateError extends Error {
    scale;
    missing;
    constructor(scale, missing) {
        super(`Scale-level ${scale} gate failed — required atoms missing or not stable: ${missing.join(', ')}`);
        this.name = 'ScaleLevelGateError';
        this.scale = scale;
        this.missing = missing;
    }
}
