export const ATOM_TYPES_UPPER = [
    'CONCEPT',
    'ADR',
    'FEAT',
    'BLUEPRINT',
    'FRAME',
    'AUDIT',
    'PROTO',
];
export class CandidateIdError extends Error {
    constructor(id) {
        super(`Invalid proposed_id "${id}" — must match /^[A-Z]+--[A-Z0-9-]+$/`);
        this.name = 'CandidateIdError';
    }
}
export class CandidateNotFoundError extends Error {
    constructor(id) {
        super(`Candidate not found: ${id}`);
        this.name = 'CandidateNotFoundError';
    }
}
