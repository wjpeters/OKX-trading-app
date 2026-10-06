"""Explicit, read-only Demo onboarding. Requires newly entered encrypted Demo credentials."""
from pathlib import Path
import subprocess,json,os
root=Path(__file__).resolve().parents[1]
subprocess.run(['rtk','proxy','python3',str(root/'scripts/bootstrap.py')],check=True)
if not (root/'.local/provider-settings/okx_demo').is_file():raise SystemExit('Sla eerst de OKX Demo-sleutel op via Systeem.')
args=['rtk','proxy','docker','compose','-f','compose.yaml','-f','compose.demo.yaml','--profile','demo','run','--rm','--no-deps']
subprocess.run(args+['demo-onboarding'],cwd=root,check=True)
r=subprocess.run(args+['--entrypoint','cat','demo-onboarding','/onboarding/demo-snapshot.json'],cwd=root,check=True,stdout=subprocess.PIPE)
value=json.loads(r.stdout);path=root/'.local/demo-onboarding.json';fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
with os.fdopen(fd,'w') as f:json.dump(value,f,indent=2)
print('Demo-account en private kanalen uitgelezen. Rapport alleen lokaal opgeslagen in .local/demo-onboarding.json. Geen order verstuurd. Uitvoering blijft geblokkeerd.')
