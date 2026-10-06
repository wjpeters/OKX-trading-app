"""Local Apple GPU service. Credential-free environment, loopback only."""
from pathlib import Path
import subprocess,os,json,urllib.request
ROOT=Path(__file__).resolve().parents[1]
VENV=ROOT/'.local/laya-venv';PID=ROOT/'.local/laya.pid';LOG=ROOT/'.local/laya.log'
try:
 with urllib.request.urlopen('http://127.0.0.1:8092/health',timeout=2)as r:
  if json.load(r).get('checkpoint')=='7b928d828b7b0e022f929d9bd2e44165aa270148':print('Lokale Laya-service draait al');raise SystemExit(0)
except (OSError,ValueError):pass
if not (VENV/'bin/python').exists():
 subprocess.run(['rtk','proxy','python3','-m','venv',str(VENV)],check=True)
 subprocess.run(['rtk','proxy',str(VENV/'bin/pip'),'install','-r',str(ROOT/'trainer/laya-requirements.txt')],check=True)
env={'PATH':str(VENV/'bin')+':/usr/bin:/bin','PYTHONUNBUFFERED':'1','OMP_NUM_THREADS':'2','TOKENIZERS_PARALLELISM':'false','LAYA_MODEL_DIR':str(ROOT/'.local/laya-models'),'LAYA_BIND':'127.0.0.1','USE_TF':'0','HF_HUB_DISABLE_TELEMETRY':'1','HF_HUB_DISABLE_XET':'1'}
with open(LOG,'ab')as f:p=subprocess.Popen([str(VENV/'bin/python'),str(ROOT/'trainer/laya_service.py')],cwd=ROOT,env=env,stdin=subprocess.DEVNULL,stdout=f,stderr=f,start_new_session=True)
PID.write_text(str(p.pid));print('Lokale Laya-service gestart; Apple GPU indien beschikbaar')
