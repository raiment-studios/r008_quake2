#!/usr/bin/env -S deno run --allow-all
import { shell, print } from './shell.ts';

const [text] = shell.args();
print.notice(text);
