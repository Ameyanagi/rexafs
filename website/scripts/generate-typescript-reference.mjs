import ts from 'typescript';
import { execFileSync } from 'node:child_process';
import { readFileSync, mkdirSync, writeFileSync, readdirSync, unlinkSync } from 'node:fs';
import { resolve } from 'node:path';
const root=resolve(import.meta.dirname,'../..');
const {tag}=JSON.parse(readFileSync(resolve(root,'website/src/data/release.json'),'utf8'));
const source=(path,channel)=>channel==='stable'?execFileSync('git',['show',`${tag}:${path}`],{cwd:root,encoding:'utf8'}):readFileSync(resolve(root,path),'utf8');
const docs=node=>(node.jsDoc??[]).map(d=>d.getText().replace(/^\/\*\*|\*\/$/g,'').split('\n').map(l=>l.replace(/^\s*\* ?/,'')).join('\n').trim()).join('\n\n');
for(const channel of ['stable','next']) {
  const dir=resolve(root,`website/src/content/docs/docs/reference/${channel}/typescript`);mkdirSync(dir,{recursive:true});
  for(const old of readdirSync(dir).filter(name=>name.endsWith('.md')))unlinkSync(resolve(dir,old));
  let count=0;
  for(const path of ['js-rexafs/types.d.ts','js-rexafs/index.d.ts','js-rexafs/node.d.ts']) {
    const file=ts.createSourceFile(path,source(path,channel),ts.ScriptTarget.Latest,true);
    for(const node of file.statements) {
      if(!ts.isClassDeclaration(node)&&!ts.isInterfaceDeclaration(node)&&!ts.isTypeAliasDeclaration(node)&&!ts.isFunctionDeclaration(node)) continue;
      const name=node.name?.text;if(!name)continue;
      const key=name==='init'?(path.includes('node.d.ts')?'init-node':'init-browser'):name.toLowerCase();
      const lines=[`---\ntitle: "TypeScript · ${name}"\ndescription: "${name} declarations and JSDoc."\naudience: user\npagefind: ${channel==='stable'}\n---\n`,
        `**${channel==='stable'?'Stable '+tag.slice(1):'Next API · unreleased'}.** ${channel==='stable'?'These signatures match the released npm package.':'These signatures describe the source checkout, not npm rexafs@0.2.4.'}`,
        '[Installation and version guide](/docs/reference/) · [TypeScript tutorial](/docs/libraries/typescript/)',docs(node),
        `[Declaration source](https://github.com/Ameyanagi/rexafs/blob/${channel==='stable'?tag:'main'}/${path})`];
      if(node.members) {
        for(const member of node.members) {
          const label=ts.isConstructorDeclaration(member)?'constructor':member.name?.getText(file);
          if(!label)continue;
          lines.push(`## ${label}`,`\`\`\`typescript\n${member.getText(file)}\n\`\`\``,docs(member));count++;
        }
      } else lines.push(`\`\`\`typescript\n${node.getText(file)}\n\`\`\``);
      writeFileSync(resolve(dir,key+'.md'),lines.filter(Boolean).join('\n\n').replace(/(?<![<(])(https?:\/\/[^\s<>]+)(?=\s)/g,'<$1>')+'\n');
    }
  }
  console.log(`Generated TypeScript ${channel}: ${count} members`);
}
