import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const excluded = new Set([".git", "node_modules", ".svelte-kit", "build", "target", "gen"]);
const findings = [];
let checked = 0;
const credentialPatterns = [
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/,
  /\b(?:ghp_|github_pat_)[A-Za-z0-9_]{30,}\b/,
  /\bAKIA[0-9A-Z]{16}\b/,
  /\bsk-(?:proj-)?[A-Za-z0-9_-]{40,}\b/,
];
async function walk(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && excluded.has(entry.name)) continue;
    const absolute = path.join(directory, entry.name);
    const relative = path.relative(root, absolute);
    if (entry.isSymbolicLink()) {
      findings.push(`${relative}: unexpected symbolic link`);
    } else if (entry.isDirectory()) {
      await walk(absolute);
    } else {
      checked++;
      if (/\.(?:db(?:-.*)?|sqlite3?(?:-.*)?|keystore|jks|pem|p12|pfx|apk|aab|exe|msi|log|zip)$/i.test(entry.name)
          || (/^\.env(?:\.|$)/.test(entry.name) && entry.name !== ".env.example")) {
        findings.push(`${relative}: runtime data, secret file or binary release`);
      }
      const bytes = await readFile(absolute);
      if (bytes.includes(0)) continue;
      const text = bytes.toString("utf8");
      if (credentialPatterns.some(pattern => pattern.test(text))) {
        findings.push(`${relative}: credential-like content (value redacted)`);
      }
      if (/[A-Z]:[\\/]+Users[\\/]+(?!Public\b|Example\b|<)[^\s"'`<>\\/]+/i.test(text)) {
        findings.push(`${relative}: personal Windows profile path`);
      }
    }
  }
}
await walk(root);
if (findings.length) {
  console.error(findings.join("\n"));
  process.exitCode = 1;
} else {
  console.log(`Public-source check passed (${checked} files). This is a heuristic scan, not a security guarantee.`);
}
