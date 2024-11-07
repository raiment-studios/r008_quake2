#!/usr/bin/env -S deno run --allow-all
import { shell, print } from './shell.ts';
import { distance } from 'https://deno.land/x/fastest_levenshtein/mod.ts';

function bestMatches(dirs: string[], term: string): string[] {
    term = term.toLocaleLowerCase();

    const checks: ((dir: string) => boolean)[] = [
        // Exact match on any segment
        (dir) => {
            const parts = dir.split('/');
            for (const part of parts) {
                if (part.toLocaleLowerCase() === term) {
                    return true;
                }
            }
            return false;
        },
        // Prefix match on any segment
        (dir) => {
            const parts = dir.split('/');
            for (const part of parts) {
                if (part.toLocaleLowerCase().startsWith(term)) {
                    return true;
                }
            }
            return false;
        },
        // Substring match
        (dir) => {
            return dir.toLocaleLowerCase().includes(term);
        },
    ];

    for (const check of checks) {
        const matches = dirs.filter(check).sort((a, b) => {
            const d = a.length - b.length;
            if (d !== 0) {
                return d;
            }
            return a.localeCompare(b);
        });

        if (matches.length > 0) {
            return matches;
        }
    }

    // Sort by Levenshtein distance
    const sorted = dirs.map((dir) => {
        const minDist = (): number => {
            let min = Infinity;
            if (dir.length < term.length) {
                return min;
            }
            for (let i = 0; i < dir.length - term.length; i++) {
                const sub = dir.toLocaleLowerCase().substr(i, term.length);
                const d = distance(sub, term);
                min = Math.min(min, d);
            }
            return min;
        };

        return {
            dir,
            distance: minDist(),
        };
    });
    sorted.sort((a, b) => {
        let d = a.distance - b.distance;
        if (d !== 0) {
            return d;
        }
        d = a.dir.length - b.dir.length;
        if (d !== 0) {
            return d;
        }
        return a.dir.localeCompare(b.dir);
    });

    return sorted.slice(0, 10).map((x) => x.dir);
}

async function readGitDirectories(): Promise<string[]> {
    const root = await shell.gitRootDirectory();
    const cmd = new Deno.Command('git', {
        args: ['ls-files'],
        cwd: root,
        stdout: 'piped',
    });
    const proc = cmd.spawn();
    const output = await proc.output();
    const files = new TextDecoder().decode(output.stdout);

    const dirs = new Set<string>();
    for (const file of files.split('\n').filter((x) => !!x)) {
        const parts = file.split('/');
        parts.pop();
        while (parts.length > 0) {
            dirs.add(parts.join('/'));
            parts.pop();
        }
    }
    const filtered = [...dirs].filter(
        (d: string) => d.length > 0 && !d.includes('/.') && !d.startsWith('.')
    );
    return filtered;
}

async function findMatches(cwd: string, term: string): Promise<string[]> {
    const aliasTable: Record<string, string> = {
        '.': cwd,
        '..': cwd.split('/').slice(0, -1).join('/'),
        '': '',
        '/': '',
        sn: `/projects/snowfall`,
        gb: `/projects/guidebook`,
        guidebook: `/projects/guidebook`,
    };
    const alias = aliasTable[term];
    if (alias !== undefined) {
        return [alias];
    }

    const dirs = await readGitDirectories();
    return bestMatches(dirs, term);
}

function parseArguments(args: string[]) {
    const options: Record<string, string | number | boolean> = {};
    const parameters: string[] = [];
    for (const arg of args) {
        const table: [RegExp, (match: RegExpMatchArray, ...args: string[]) => void][] = [
            [
                /^-([a-zA-Z])$/,
                (_m, letter) => {
                    options[letter] = true;
                },
            ],
            [
                /^--([a-zA-Z][a-zA-Z\-_]+)=(.+)$/,
                (_m, key, value) => {
                    options[key] = value;
                },
            ],
            [
                /^--([a-zA-Z][a-zA-Z\-_]+)$/,
                (_m, key) => {
                    options[key] = true;
                },
            ],
            [
                /^(.+)$/,
                (_m, value) => {
                    parameters.push(value);
                },
            ],
        ];
        for (const [pattern, handler] of table) {
            const match = arg.match(pattern);
            if (match) {
                handler(match, ...match.slice(1));
                break;
            }
        }
    }
    return { options, parameters };
}

function readStack(): string[] {
    const text = localStorage.getItem('rcd:stack');
    if (!text) {
        return [];
    }
    try {
        const arr = JSON.parse(text);
        return arr;
    } catch (_e) {
        return [];
    }
}

function writeStack(stack: string[]) {
    // Keep the most recent 32 entries while droppping older duplicates.
    const set = new Set();
    const recent = stack.slice(-32);
    const filtered: string[] = [];
    for (let i = recent.length - 1; i >= 0; i--) {
        const entry = recent[i];
        if (set.has(entry)) {
            continue;
        }
        set.add(entry);
        filtered.unshift(entry);
    }
    localStorage.setItem('rcd:stack', JSON.stringify(filtered));
}

async function main(args: string[]) {
    const { options, parameters } = parseArguments(args);

    if (options.p) {
        options.pop = true;
        delete options.p;
    }
    if (options.c) {
        options.clear = true;
        delete options.c;
    }
    if (options.v) {
        options.verbose = true;
        delete options.v;
    }

    const cwd = Deno.cwd();
    const root = await shell.gitRootDirectory();
    const relCWD = cwd.replace(`${root}/`, '');

    const control = {
        updateStack: true,
        showContext: false,
    };

    const matches: string[] = await (async () => {
        for (const [key, _value] of Object.entries(options)) {
            switch (key) {
                case 'verbose': {
                    control.showContext = true;
                    delete options[key];
                    break;
                }
            }
        }

        for (const [key, _value] of Object.entries(options)) {
            switch (key) {
                case 'clear': {
                    control.updateStack = false;
                    console.warn('Clearing local storage');
                    localStorage.removeItem('rcd:stack');
                    return [''];
                }
                case 'pop': {
                    const stack = readStack();
                    let last = stack.length === 0 ? '' : (stack.pop() as string);
                    if (last === relCWD && stack.length > 0) {
                        last = stack.pop() as string;
                    }
                    writeStack(stack);
                    return [last];
                }
                default:
                    console.warn(`Unknown option: ${key}`);
                    return [];
            }
        }

        const term = parameters[0];
        return await findMatches(relCWD, term);
    })();

    if (control.showContext) {
        print.cwarn('882', `date:    ${new Date().toISOString()}`);
        print.cwarn('882', `gitroot: ${root}`);
        print.cwarn('882', `cwd:     ${relCWD}`);
        print.cwarn('882', `terms:   ${parameters.join(' ')}`);
        print.cwarn('882', `matches: ${matches.length}`);
    }

    if (matches.length === 0) {
        console.warn(`No matches found`);
        console.log(cwd);
        Deno.exit(1);
    }

    if (matches.length > 0) {
        print.cwarn('3C5', `→ ${matches[0] || '/'}`);
        for (const match of matches.slice(1, 5)) {
            print.cwarn('555', `  ${match}`);
        }
    }

    const result = `${matches[0]}`;
    const stack = readStack();
    if (control.updateStack) {
        stack.push(result);
        writeStack(stack);
    }

    {
        const recent = readStack().slice(-3).reverse();
        for (let i = 0; i < recent.length; i++) {
            print.cwarn('337', `${i} ${recent[i] || '/'}`);
        }
    }

    for (const match of matches) {
        console.log(`${root}/${match}`);
    }
}

// It can be annoying if an error in this development script causes
// the caller to switch to the root directory, so create a fallback
// so on errors the script is effectively telling the caller to stay
// put.  This is arguably partially masking an error that should never
// have occurred -- but this is a convenience script, not
// production-grade code.
globalThis.addEventListener('unhandledrejection', (e: any) => {
    console.log(Deno.cwd());
    throw e;
});

main(Deno.args);
