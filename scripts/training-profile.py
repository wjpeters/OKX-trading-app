"""One durable research start request. Public data / local training API only.
No account configuration, qualification, releases or orders are performed.
"""
import json,time,sys,os,fcntl,subprocess,urllib.request,urllib.error
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];LOCAL=ROOT/'.local'
FILE=LOCAL/'training-profile.json';PID=LOCAL/'training-profile.pid';LOG=LOCAL/'training-profile.log'
BASE='http://127.0.0.1:4780'
def save(value):
 tmp=FILE.with_suffix('.tmp');tmp.write_text(json.dumps(value,indent=2));os.replace(tmp,FILE)
def api(path,body=None):
 req=urllib.request.Request(BASE+path,data=None if body is None else json.dumps(body).encode(),headers={'Origin':BASE,'Content-Type':'application/json','X-Terminal-Request':'paper-v2'})
 with urllib.request.urlopen(req,timeout=20)as r:return json.load(r)
def run():
 lock=open(LOCAL/'training-profile.lock','a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);PID.write_text(str(os.getpid()))
 while True:
  plan=json.loads(FILE.read_text())
  if plan.get('status')in ('STARTED','FAILED','CANCELED'):return
  try:
   state=api('/api/training');body=plan['request']
   existing=next((c for c in state['campaigns']if c['config']['name']==body['name']and [d['id']for d in c['config']['datasets']]==body['dataset_ids']),None)
   if existing:plan.update(status='STARTED',campaign_id=existing['id']);save(plan);return
   datasets=[next(d for d in state['datasets']if d['id']==id)for id in body['dataset_ids']]
   if any(d['status']=='INCOMPLETE'for d in datasets):plan.update(status='FAILED',reason='Twaalf maanden ononderbroken historie ontbreken');save(plan);return
   for d in datasets:
    if d['status']in ('INTERRUPTED','ERROR'):api('/api/training/datasets/'+d['id']+'/resume',{})
   if all(d['status']=='READY'for d in datasets)and state['laya']['ready']:
    plan.update(status='START_REQUESTED');save(plan)
    campaign=api('/api/training/campaigns',body);plan.update(status='STARTED',campaign_id=campaign['id'],reason=None);save(plan);return
   plan.update(status='WAITING_DATA',reason='Volledige historie en vastgezet lokaal Laya-model vereist');save(plan)
  except (OSError,ValueError,StopIteration)as e:
   plan['last_error']=str(e);save(plan)
  time.sleep(30)
def start():
 if not FILE.exists()or json.loads(FILE.read_text()).get('status')in ('STARTED','FAILED','CANCELED'):return
 env={'PATH':os.environ.get('PATH','/usr/bin:/bin'),'PYTHONUNBUFFERED':'1'}
 with open(LOG,'ab')as log:p=subprocess.Popen([sys.executable,str(Path(__file__).resolve()),'run'],cwd=ROOT,env=env,stdin=subprocess.DEVNULL,stdout=log,stderr=log,start_new_session=True)
 print('Onderzoeksstart wacht op volledige datasets; geen orders')
def stop():
 if PID.exists():
  pid=int(PID.read_text());cmd=subprocess.run(['rtk','proxy','ps','-o','command=','-p',str(pid)],capture_output=True,text=True).stdout
  if str(Path(__file__).resolve())in cmd:os.kill(pid,15)
  PID.unlink()
if __name__=='__main__':{'start':start,'run':run,'stop':stop}[sys.argv[1]]()
