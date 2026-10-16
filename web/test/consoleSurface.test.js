import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const srcRoot = join(dirname(fileURLToPath(import.meta.url)), '../src');

function sourceFiles(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.(css|svelte|js)$/.test(name) ? [path] : [];
  });
}

function read(name) {
  return readFileSync(join(srcRoot, name), 'utf8');
}

test('console surface uses charcoal page, copper action, and copper focus', () => {
  const css = read('app.css');
  const app = read('App.svelte');

  assert.match(css, /background-color:\s*#1b1d1f/i);
  assert.match(app, /bg-\[#1b1d1f\]/i);
  assert.match(css, /\.btn-primary\s*\{[^}]*background:\s*#e08a2c/i);
  assert.match(css, /\.btn-primary\s*\{[^}]*color:\s*#1a1208/i);
  assert.match(css, /outline:\s*2px solid #e08a2c/i);
});

test('console surface leaves no cyan in the loaded sources', () => {
  const hits = sourceFiles(srcRoot).flatMap((path) => {
    const lines = readFileSync(path, 'utf8').split('\n');
    return lines.flatMap((line, index) =>
      /cyan|indigo|#22d3ee|#67e8f9|#090d16/i.test(line)
        ? [`${relative(srcRoot, path)}:${index + 1}: ${line.trim()}`]
        : [],
    );
  });

  assert.deepEqual(hits, []);
});

test('console surface maps slate utilities onto the charcoal ramp', () => {
  const css = read('app.css');
  assert.match(css, /--color-slate-950:\s*#141618/i);
  assert.match(css, /--color-slate-900:\s*#242628/i);
  assert.match(css, /--color-slate-100:\s*#e6e4df/i);
  assert.match(css, /--color-slate-400:\s*#a39e96/i);
  assert.match(css, /--color-slate-700:\s*#3c3a36/i);
});

test('navbar includes exit buttons for both mobile and desktop viewports', () => {
  const navbar = read('components/Navbar.svelte');
  assert.match(navbar, /btn-icon\s+md:hidden/);
  assert.match(navbar, /btn-icon\s+hidden\s+md:inline-flex/);
});

test('component classes are defined in @layer components so utilities can override display', () => {
  const css = read('app.css');
  assert.match(css, /@layer\s+components\s*\{[\s\S]*\.btn\s*\{/);
});
