export class SlmError extends Error {
    kind;
    cause;
    constructor(message, kind, cause) {
        super(message);
        this.kind = kind;
        this.cause = cause;
        this.name = 'SlmError';
    }
}
