#!/usr/bin/env node
// Brand-neutral guard: fail when legacy brand identifiers leak into app sources.
//
// Scope deliberately excludes documentation, README, rules/ and generated
// artifacts. `tests/` fixtures are whitelisted because a fixture may need to
// reproduce legacy upstream responses verbatim.
import { readdir, readFile, stat } from "node:fs/promises";
import { dirname, extname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SCOPE = ["src", "src-tauri", "index.html", "package.json", "scripts", ".github/workflows"];
const WHITELIST_PREFIXES = ["tests/"];
const WHITELIST_FILES = ["scripts/test-brand-neutral.mjs"];
const SKIP_DIRS = new Set(["node_modules", ".git", "target", "dist", "test-results"]);
// Text-like extensions only. Images, archives, compiled binaries are skipped.
const TEXT_EXTENSIONS = new Set([
  ".c", ".cc", ".cfg", ".conf", ".cpp", ".css", ".gradle", ".h", ".hpp",
  ".htm", ".html", ".ini", ".java", ".js", ".json", ".jsx", ".kts", ".kt", ".lock",
  ".md", ".mjs", ".cjs", ".plist", ".properties", ".py", ".rs", ".scss",
  ".sh", ".sql", ".svg", ".toml", ".ts", ".tsx", ".txt", ".vue", ".xml",
  ".yaml", ".yml",
]);
const MAX_FILE_BYTES = 2 * 1024 * 1024;
// Keep the terms split so this file itself does not need a self-match exemption
// beyond the explicit WHITELIST_FILES entry above.
const TERMS = ["黄果", "huangguo", "adihsppe"];
const BRAND_PATTERN = new RegExp(TERMS.join("|"), "gi");

const asPosix = value => value.split(sep).join("/");
const isWhitelisted = rel => {
  const normalized = asPosix(rel);
  return WHITELIST_PREFIXES.some(prefix => normalized.startsWith(prefix)) ||
    WHITELIST_FILES.includes(normalized);
};

async function collectFiles(entry) {
  const absolute = join(root, entry);
  let info;
  try {
    info = await stat(absolute);
  } catch {
    return [];
  }
  if (info.isFile()) return [absolute];
  if (!info.isDirectory()) return [];
  const files = [];
  const queue = [absolute];
  while (queue.length > 0) {
    const current = queue.pop();
    let entries;
    try {
      entries = await readdir(current, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const item of entries) {
      if (item.isDirectory()) {
        if (!SKIP_DIRS.has(item.name)) queue.push(join(current, item.name));
      } else if (item.isFile()) {
        files.push(join(current, item.name));
      }
    }
  }
  return files;
}

function isProbablyText(buffer) {
  const sample = buffer.subarray(0, Math.min(buffer.length, 8192));
  return !sample.includes(0);
}

function excerptFor(line) {
  const compact = line.trim().replace(/\s+/g, " ");
  return compact.length > 240 ? `${compact.slice(0, 237)}...` : compact;
}

const files = new Map();
for (const entry of SCOPE) {
  for (const file of await collectFiles(entry)) {
    const rel = asPosix(relative(root, file));
    if (isWhitelisted(rel)) continue;
    if (!TEXT_EXTENSIONS.has(extname(file).toLowerCase())) continue;
    files.set(rel, file);
  }
}

const matches = [];
let scanned = 0;
for (const [rel, file] of [...files.entries()].sort(([a], [b]) => a.localeCompare(b))) {
  let buffer;
  try {
    const info = await stat(file);
    if (info.size > MAX_FILE_BYTES) continue;
    buffer = await readFile(file);
  } catch {
    continue;
  }
  if (!isProbablyText(buffer)) continue;
  scanned += 1;
  const text = buffer.toString("utf8");
  text.split(/\r?\n/).forEach((line, index) => {
    BRAND_PATTERN.lastIndex = 0;
    let match;
    while ((match = BRAND_PATTERN.exec(line)) !== null) {
      matches.push({ file: rel, line: index + 1, column: match.index + 1, term: match[0], excerpt: excerptFor(line) });
      if (match[0].length === 0) BRAND_PATTERN.lastIndex += 1;
    }
  });
}

const jsonMode = process.argv.includes("--json");
if (jsonMode) {
  console.log(JSON.stringify({ root, scopes: SCOPE, terms: TERMS, filesScanned: scanned, matches }, null, 2));
} else {
  if (matches.length > 0) {
    for (const match of matches) {
      console.error(`${match.file}:${match.line}:${match.column}  [${match.term}]  ${match.excerpt}`);
    }
    console.error(`Brand-neutral scan FAILED: ${matches.length} hit(s) in ${new Set(matches.map(m => m.file)).size} file(s) (${scanned} text files scanned).`);
  } else {
    console.log(`Brand-neutral scan passed: 0 hit(s) across ${scanned} text file(s).`);
  }
}
if (matches.length > 0) process.exitCode = 1;
