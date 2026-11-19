// Lighthouse mobile audit of the shell page against the release build of `web`.
// Thresholds come from AAO-014: Performance >= 90 and Accessibility >= 95 (median of RUNS runs,
// because a single Lighthouse run varies a few points). Usage: `just lighthouse`.
import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const port = process.env.LH_PORT ?? '8098';
const origin = `http://127.0.0.1:${port}`;
const runs = Number.parseInt(process.env.LH_RUNS ?? '3', 10);
const thresholds = { performance: 90, accessibility: 95 };
const pages = (process.env.LH_PATHS ?? '/,/en').split(',');
const out = join(here, '.lighthouse');
mkdirSync(out, { recursive: true });

const server = spawn(join(here, '..', 'target', 'release', 'web'), {
  env: { ...process.env, WEB_BIND: `127.0.0.1:${port}`, RUST_LOG: 'warn' },
  stdio: 'inherit',
});
const stop = () => server.kill('SIGTERM');
process.on('exit', stop);

async function waitForServer() {
  for (let i = 0; i < 100; i += 1) {
    try {
      if ((await fetch(`${origin}/healthz`)).ok) return;
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('web did not become healthy');
}

const median = (xs) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];
const lighthouse = join(here, 'node_modules', '.bin', 'lighthouse');
let failed = false;

try {
  await waitForServer();
  for (const path of pages) {
    const scores = { performance: [], accessibility: [] };
    for (let run = 0; run < runs; run += 1) {
      const file = join(out, `${path.replaceAll('/', '_') || '_'}-${run}.json`);
      execFileSync(
        lighthouse,
        [
          `${origin}${path}`,
          '--quiet',
          '--output=json',
          `--output-path=${file}`,
          '--only-categories=performance,accessibility',
          '--chrome-flags=--headless=new --no-sandbox --disable-gpu',
        ],
        { stdio: ['ignore', 'inherit', 'inherit'], env: process.env },
      );
      const { categories } = JSON.parse(readFileSync(file, 'utf8'));
      for (const key of Object.keys(scores)) scores[key].push(Math.round(categories[key].score * 100));
    }
    for (const [key, min] of Object.entries(thresholds)) {
      const score = median(scores[key]);
      const ok = score >= min;
      failed ||= !ok;
      console.log(`${ok ? 'PASS' : 'FAIL'} ${path} ${key}: ${score} (runs ${scores[key].join(', ')}; need >= ${min})`);
    }
  }
} finally {
  stop();
}
process.exit(failed ? 1 : 0);
