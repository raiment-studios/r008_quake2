#!/usr/bin/env -S deno run --allow-all
import { ensure, shell } from './shell.ts';

const targets = shell.args();
const dirname = await shell.gitRelativeDirectory();

// Create a nicely formatted title
const segments = dirname.split('/');
const folder = segments.pop();
const title = `${folder}` + (segments.length > 0 ? ` (${segments.join('/')})` : '');

await shell.spawn('zellij', ['action', 'rename-pane', title]);
await ensure.spawn('mprocs', [
    '--names',
    targets.join(','),
    ...targets.map((target) => `make ${target}`),
]);
await shell.spawn('zellij', ['action', 'rename-pane', '']);
