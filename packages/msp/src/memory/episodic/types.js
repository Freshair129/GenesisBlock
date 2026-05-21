export class EpisodicSchemaError extends Error {
    reason;
    constructor(reason) {
        super(`episodic schema: ${reason}`);
        this.reason = reason;
        this.name = 'EpisodicSchemaError';
    }
}
