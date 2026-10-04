"""Project/image inspection runs with --network none. Public databases downloaded separately."""
import subprocess,tempfile,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];EVIDENCE=ROOT/'evidence'
TRIVY='aquasec/trivy@sha256:af6acf9a6b85dfe389a1941505c0ce9efef52a4719635e1a962f022a3d855daa'
for tag,name in [('okx-tradingterminal:0.4.0','terminal'),('okx-pine-worker:0.3.0','pine'),('okx-postgres:18.6','postgres')]:
 with tempfile.TemporaryDirectory(prefix='okx-local-scan-') as tmp:
  subprocess.run(['rtk','proxy','docker','save','-o',tmp+'/image.tar',tag],check=True)
  subprocess.run(['rtk','proxy','docker','run','--rm','--network','none','--read-only','--cap-drop=ALL','--memory=1g','--cpus=2','--tmpfs','/tmp:size=512m','-v','okx-trivy-db:/root/.cache/trivy','-v',tmp+':/input:ro','-v',str(EVIDENCE)+':/reports',TRIVY,'image','--input','/input/image.tar','--skip-db-update','--skip-java-db-update','--offline-scan','--scanners','vuln','--no-progress','--format','json','--output','/reports/'+name+'-image.json'],check=True)
  r=json.loads((EVIDENCE/(name+'-image.json')).read_text());v=[v for x in r.get('Results',[]) for v in x.get('Vulnerabilities',[])];counts={s:sum(x['Severity']==s for x in v) for s in ['CRITICAL','HIGH','MEDIUM','LOW','UNKNOWN']};print(json.dumps({'image':tag,'counts':counts,'network':'none','metadata_uploaded':False}),flush=True)
