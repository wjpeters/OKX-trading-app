"""Builds/tests the same musl source as the runtime; evidence stays local."""
import subprocess,json
from pathlib import Path
r=Path(__file__).resolve().parents[1]
def run(args,file):
 with (r/'evidence'/file).open('w') as f:
  p=subprocess.run(['rtk','proxy',*args],cwd=r,stdout=f,stderr=subprocess.STDOUT)
 print(json.dumps({'check':file,'exit_code':p.returncode}),flush=True)
 if p.returncode:
  print((r/'evidence'/file).read_text()[-6000:]);raise SystemExit(p.returncode)
run(['docker','build','--target','verification','--progress=plain','.'],'phase4-unit-tests.txt')
run(['docker','run','--rm','--network','none','--read-only','--cap-drop=ALL','--memory=256m','--cpus=.5','--tmpfs','/tmp:size=8m','okx-pine-worker:0.3.0','node','/app/tests.mjs'],'pine-tests.json')
run(['docker','run','--rm','-v',str(r/'backend')+':/work:ro','-v','okx-terminal-cargo:/usr/local/cargo','-w','/work','rust@sha256:452176c0cefca88c0b3184ce85a4eb03e3d4fa05d2afb5366abcba853221019e','cargo','audit','--json'],'rust-advisories.json')
