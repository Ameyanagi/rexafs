/** Keep released declarations, with reviewed explanations from editor JSDoc. */
import ts from 'typescript';
import { execFileSync } from 'node:child_process';
import { readFileSync, mkdirSync, writeFileSync, readdirSync, unlinkSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const defaultRoot = resolve(import.meta.dirname, '../..');
const source = (root, tag, path, channel) => channel === 'stable'
  ? execFileSync('git', ['show', `${tag}:${path}`], { cwd: root, encoding: 'utf8' })
  : readFileSync(resolve(root, path), 'utf8');
const docs = node => (node.jsDoc ?? []).map(d => d.getText().replace(/^\/\*\*|\*\/$/g, '')
  .split('\n').map(l => l.replace(/^\s*\* ?/, '')).join('\n').trim()).join('\n\n');
const publicNode = node => !node.modifiers?.some(m => m.kind === ts.SyntaxKind.PrivateKeyword || m.kind === ts.SyntaxKind.ProtectedKeyword);
const label = node => ts.isConstructorDeclaration(node) ? 'constructor' : node.name?.getText();
const supported = node => ts.isClassDeclaration(node) || ts.isInterfaceDeclaration(node)
  || ts.isTypeAliasDeclaration(node) || ts.isFunctionDeclaration(node);
function helpFor(entries, name, context) {
  const node = entries.get(name);
  const help = node && docs(node);
  if (!help) throw Error(`Missing source JSDoc: ${context}.${name}`);
  return help;
}

/**
 * Generate both channels from declarations and their maintained JSDoc.
 *
 * Stable signatures come from the checkout's release tag; explanations come
 * from its current declarations. Missing public help throws. Only generated
 * TypeScript pages are replaced. A separate checkout can be supplied for tests.
 */
export function generate(root = defaultRoot) {
  const { tag } = JSON.parse(readFileSync(resolve(root, 'website/src/data/release.json'), 'utf8'));
  for (const channel of ['stable', 'next']) {
    const dir = resolve(root, `website/src/content/docs/docs/reference/${channel}/typescript`);
    mkdirSync(dir, { recursive: true });
    for (const old of readdirSync(dir).filter(name => name.endsWith('.md'))) unlinkSync(resolve(dir, old));
    let count = 0;
    for (const path of ['js-rexafs/types.d.ts', 'js-rexafs/index.d.ts', 'js-rexafs/node.d.ts']) {
      const file = ts.createSourceFile(path, source(root, tag, path, channel), ts.ScriptTarget.Latest, true);
      const currentFile = ts.createSourceFile(path, source(root, tag, path, 'next'), ts.ScriptTarget.Latest, true);
      const current = new Map(currentFile.statements.filter(supported).map(node => [node.name?.text, node]));
      for (const node of file.statements) {
        if (!supported(node)) continue;
        const name = node.name?.text;
        if (!name) continue;
        const key = name === 'init' ? (path.includes('node.d.ts') ? 'init-node' : 'init-browser') : name.toLowerCase();
        const lines = [
          `---\ntitle: "TypeScript · ${name}"\ndescription: "${name} declarations, defaults and API explanations."\naudience: user\npagefind: ${channel === 'stable'}\n---`,
          `**${channel === 'stable' ? 'Stable ' + tag.slice(1) : 'Next API · unreleased'}.** ${channel === 'stable'
            ? 'These signatures match the released npm package. Explanations are maintained in source JSDoc and reviewed against this release.'
            : `This reference describes the source checkout, including additions not available in npm rexafs@${tag.slice(1)}.`}`,
          '[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)',
          `[Declaration source](https://github.com/Ameyanagi/rexafs/blob/${channel === 'stable' ? tag : 'main'}/${path}) · [JSDoc source](https://github.com/Ameyanagi/rexafs/blob/main/${path})`,
          helpFor(current, name, path),
        ];
        if (node.members) {
          const currentMembers = new Map(current.get(name).members.filter(publicNode).map(member => [label(member), member]));
          for (const member of node.members.filter(publicNode)) {
            const key = label(member);
            if (!key) continue;
            lines.push(`## ${key}`, `\`\`\`typescript\n${member.getText(file)}\n\`\`\``, helpFor(currentMembers, key, name));
            count++;
          }
        } else lines.push(`\`\`\`typescript\n${node.getText(file)}\n\`\`\``);
        writeFileSync(resolve(dir, key + '.md'), lines.join('\n\n').replace(/(?<![<(])(https?:\/\/[^\s<>]+)(?=\s|$)/g, '<$1>') + '\n');
      }
    }
    console.log(`Generated TypeScript ${channel}: ${count} members`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) generate();
