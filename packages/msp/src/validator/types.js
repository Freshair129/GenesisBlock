export class ValidatorIOError extends Error {
    cause;
    constructor(message, cause) {
        super(message);
        this.cause = cause;
        this.name = 'ValidatorIOError';
    }
}
