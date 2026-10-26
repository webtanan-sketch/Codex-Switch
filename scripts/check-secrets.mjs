import {execFileSync} from 'node:child_process';
import {readFileSync, readdirSync, existsSync} from 'node:fs';
function walk(dir = '.') {
 const skip = new Set(['.git','node_modules','target','dist','gen','release-artifacts']);
 return readdirSync(dir,{withFileTypes:true}).flatMap(e => {
  if(skip.has(e.name) || e.isSymbolicLink()) return [];
  const path = dir === '.' ? e.name : `${dir}/${e.name}`;
  return e.isDirectory() ? walk(path) : [path];
 });
}
const files = existsSync('.git') ? execFileSync('git',['ls-files','--cached','--others','--exclude-standard','-z'],{encoding:'utf8'}).split('\0').filter(Boolean) : walk();
let failed = false;
for(const file of files) {
  if(/(^|\/)(auth\.json(?:\.|$)|vault\/|\.env$)/.test(file)) { console.error(`Forbidden credential path: ${file}`); failed = true; }
  if(!/\.(?:rs|ts|tsx|mjs|cjs|json|md|yml|toml|ps1|html|css|txt)$/.test(file)) continue;
  const content = readFileSync(file,'utf8');
  if(/\beyJ[A-Za-z0-9_-]{15,}\.[A-Za-z0-9_-]{15,}\.[A-Za-z0-9_-]{15,}\b/.test(content) || /\bsk-(?:proj-)?[A-Za-z0-9_-]{28,}\b/.test(content)) { console.error(`Potential credential in: ${file}`); failed = true; }
}
if(failed) process.exitCode = 1; else console.log(`Credential pattern scan passed (${files.length} source paths).`);
