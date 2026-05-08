# Release Gate

Scope: local QA/release readiness review for the AiPlus Auto Compact
subproduct/module, installer-path, and auto-resume documentation update.

## Gate Status

PASS.

Local validation passed before the approved GitHub `main` update. GitHub
remote/latest-commit verification must be repeated after this update is pushed
to `https://github.com/izhiwen/aiplus-auto-compact`.

## Required Checks

| Gate | Status | Evidence |
| --- | --- | --- |
| `core/scripts/compactctl.mjs` syntax | PASS | `rtk node --check core/scripts/compactctl.mjs`, exit 0 |
| Test suite | PASS | `rtk npm test`, 15/15 acceptance tests passed |
| AiPlus CLI alignment | PASS | `rtk cargo test` from the sibling `../aiplus-rust` checkout, 6/6 tests passed; `cargo run -p aiplus-cli -- --help` shows `compact` |
| Manifest/config JSON parse | PASS | Parsed package metadata, Codex plugin manifest, Claude plugin manifest, Claude hook example, and OpenCode config example |
| Adapter required files | PASS | Required Codex, Claude Code, and OpenCode adapter files exist |
| README runtime coverage | PASS | Root README covers AiPlus CLI (`aiplus`), Codex, Claude Code, OpenCode, validation, safety, and current release status |
| README beginner flow | PASS | Root README and `README.zh-CN.md` cover AiPlus ecosystem installer path, existing `aiplus` path, module-only path, `刷新`/`refresh`, and `aiplus compact ...` before legacy Node reference |
| Subproduct identity | PASS | README states AiPlus Auto Compact is an independent AiPlus subproduct and bundled AiPlus CLI module |
| Best-effort auto-resume | PASS | README, core docs, adapter docs, and examples describe host-return resume behavior and user-message fallback |
| README link/path sanity | PASS | Relative markdown links checked with no broken links |
| Public-safety scan | PASS | No blocker; expected policy/detector/test matches only |
| Private-data scan | PASS | No real private data found; expected policy/detector matches only |
| Old Codex-only branding in root/core | PASS WITH CAVEAT | `.codex/compact` compatibility paths remain intentionally documented in core; no release blocker |
| No npm publish/tag/release | PASS | No package publish, tag, GitHub Release, marketplace submission, or global install performed |
| GitHub remote URL/latest commit after push | FINAL RESULT EVIDENCE | Verify after each reviewed documentation commit is pushed to `origin/main` |

## Final Rerun Commands

Run from the final `aiplus-auto-compact` checkout before publication:

```bash
rtk node --check core/scripts/compactctl.mjs
rtk npm test
rtk node - <<'NODE'
const fs=require('fs'), path=require('path');
const want=new Set(['package.json','plugin.json','opencode.json.example','hooks.example.json']);
function walk(d,out=[]){for(const e of fs.readdirSync(d,{withFileTypes:true})){if(e.name==='.git'||e.name==='node_modules')continue; const p=path.join(d,e.name); if(e.isDirectory())walk(p,out); else if(want.has(e.name))out.push(p)} return out}
const files=walk('.').sort();
for (const f of files){JSON.parse(fs.readFileSync(f,'utf8')); console.log('OK '+f)}
NODE
rtk node - <<'NODE'
const fs=require('fs');
const required={
  'core':['scripts/compactctl.mjs','schemas/compact-policy.schema.json','templates/compact-policy.json','docs/protocol.md'],
  'adapters/codex':['.codex-plugin/plugin.json','skills/compact-protocol/SKILL.md','README.md'],
  'adapters/claude-code':['.claude-plugin/plugin.json','skills/compact-protocol/SKILL.md','commands/compact-checkpoint.md','commands/compact-resume.md','commands/compact-validate.md','README.md'],
  'adapters/opencode':['opencode.json.example','agents/compact-advisor.md','agents/compact-reviewer.md','commands/compact-checkpoint.md','commands/compact-resume.md','commands/compact-validate.md','prompts/compact-protocol.md','README.md'],
  'examples':['codex-project/README.md','claude-code-project/README.md','opencode-project/README.md']
};
let fail=false;
for(const [base,files] of Object.entries(required)){
 for(const rel of files){const p=base+'/'+rel; const ok=fs.existsSync(p); console.log((ok?'OK ':'MISSING ')+p); if(!ok) fail=true;}
}
process.exit(fail?1:0);
NODE
```

Run AiPlus CLI alignment checks from the sibling `../aiplus-rust` checkout:

```bash
rtk cargo test
rtk cargo run -p aiplus-cli -- --help
```

After GitHub publication, verify:

```bash
rtk git remote -v
rtk git log -1 --oneline
```

## Forbidden Actions

Not performed:

- npm publish
- Cargo publish
- package registry publish
- GitHub Release creation
- git tag release
- binary upload
- marketplace submission
- global install
- `$CODEX_HOME` modification
- deletion or transfer of `codex-compact-protocol`

## Handoff Notes

Owner gate needed: no for pushing reviewed `main` changes to
`izhiwen/aiplus-auto-compact` under the current approved goal. Separate Owner
approval is still required for npm/Cargo/package registry publish, tags, GitHub
Releases, binary uploads, marketplace submission, global installs, and deletion
or transfer of the existing Codex-first repo.
