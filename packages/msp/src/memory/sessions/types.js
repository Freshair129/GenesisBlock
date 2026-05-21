export class SessionLockedError extends Error {
    holderPid;
    path;
    constructor(holderPid, path) {
        super(`session lock held by pid ${holderPid} at ${path}`);
        this.holderPid = holderPid;
        this.path = path;
        this.name = 'SessionLockedError';
    }
}
export class SessionSchemaError extends Error {
    missingFields;
    constructor(missingFields) {
        super(`session row missing required fields: ${missingFields.join(', ')}`);
        this.missingFields = missingFields;
        this.name = 'SessionSchemaError';
    }
}
