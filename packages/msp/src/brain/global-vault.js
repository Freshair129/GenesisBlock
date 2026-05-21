import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
const GLOBAL_SUBDIR = {
    SKILL: 'skills',
    ALGO: 'algo',
    PROTO: 'proto',
    PARAMS: 'params',
    IDENTITY: '.',
    REGISTRY: '.',
};
export function globalRoot() {
    if (process.platform === 'win32') {
        const userProfile = process.env.USERPROFILE;
        if (!userProfile) {
            throw new Error('globalRoot: USERPROFILE is not set on win32');
        }
        return path.join(userProfile, '.brain');
    }
    const xdg = process.env.XDG_DATA_HOME;
    if (xdg) {
        return path.join(xdg, 'brain');
    }
    return path.join(os.homedir(), '.brain');
}
export function globalSubdir(type) {
    const sub = GLOBAL_SUBDIR[type];
    if (sub === undefined) {
        throw new Error(`globalSubdir: ${type} is not routed to the global brain`);
    }
    return path.join(globalRoot(), sub);
}
export async function exists() {
    try {
        const stat = await fs.stat(globalRoot());
        return stat.isDirectory();
    }
    catch {
        return false;
    }
}
