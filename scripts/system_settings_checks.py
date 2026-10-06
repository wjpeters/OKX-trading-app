#!/usr/bin/env python3
"""Credential and provider regressions in a disposable DB; no real keys or provider calls."""
import phase2_acceptance as b
import json,os,sys,urllib.request
from pathlib import Path
b.MANIFEST=b.ROOT/'.local/system-settings-test.json'
b.IMAGE='okx-tradingterminal:0.4.0'
base_create=b.create

def create(name,db,port=None):
 d=b.ROOT/'.local'/name;d.mkdir(mode=0o700,exist_ok=True)
 key=d/'master'
 if not key.exists():key.write_bytes(os.urandom(32))
 key.chmod(0o600)
 credentials=d/'credentials';credentials.mkdir(mode=0o700,exist_ok=True)
 args=['docker','create','--name',name,'--network','okx-tradingterminal_market','--read-only','--cap-drop=ALL','--security-opt=no-new-privileges:true','--memory=512m','-e','EXECUTION_MODE=PAPER_LOCAL','-e','DATABASE_HOST='+b.DB_CONTAINER,'-e','DATABASE_NAME='+db,'-e','DATABASE_PASSWORD_FILE=/run/secrets/db_password','-e','SETTINGS_DIR=/settings','-e','RUST_LOG=warn','-v',str(b.ROOT/'.local/db_password')+':/run/secrets/db_password:ro','-v',str(key)+':/run/secrets/local_secrets_key:ro','-v',str(credentials)+':/settings']
 if port:args+=['-p','127.0.0.1:'+port+':8080']
 b.run(*args,b.IMAGE);b.run('docker','network','connect','okx-tradingterminal_storage',name);b.run('docker','start',name)
b.create=create
checks=[]
def ok(name,passed):
 checks.append({'check':name,'passed':bool(passed)})
 (b.ROOT/'evidence/system-settings-integration.json').write_text(json.dumps({'scope':'disposable database; synthetic credentials; no paid AI calls or orders','checks':checks},indent=2))
 print(('PASS ' if passed else 'FAIL ')+name,flush=True)
 assert passed,name

def request(path,body=None,headers=None):
 c,v,_=b.api(path,body,headers);return c,v

def check():
 m=json.loads(b.MANIFEST.read_text());folder=b.ROOT/'.local'/m['container']/'credentials'
 user_before=json.load(urllib.request.urlopen('http://127.0.0.1:4780/api/paper'))['account']
 c,s=request('/api/settings');ok('Settings load with missing keys',c==200 and s['storage_ready'] and all(not x['configured'] for x in s['credentials'].values()))
 ok('Default provider preserves OpenRouter behavior',s['provider']=='openrouter' and not s['enabled'])
 ok('Missing provider key blocks activation',request('/api/jev/config',{'budget_usd':'1','enabled':True,'confirmed':True})[0]==422)
 fixture='synthetic-settings-fixture-not-a-real-key'
 req={'target':'openrouter_key','api_key':fixture}
 for name,headers in [('No Origin',{'Content-Type':'application/json','X-Terminal-Request':'paper-v2'}),('Foreign Origin',{'Origin':'https://example.invalid','Content-Type':'application/json','X-Terminal-Request':'paper-v2'}),('No CSRF header',{'Origin':b.BASE,'Content-Type':'application/json'})]:
  ok(name+' cannot write credentials',request('/api/settings/credentials',req,headers)[0]==403)
 ok('Unknown destination rejected',request('/api/settings/credentials',{'target':'../escape','api_key':fixture})[0]==422)
 ok('Partial OKX credentials rejected',request('/api/settings/credentials',{'target':'okx_demo','api_key':fixture})[0]==422)
 ok('Whitespace credentials rejected',request('/api/settings/credentials',{'target':'typesafe_key','api_key':'   '})[0]==422)
 ok('Unexpected fields rejected without echo',request('/api/settings/credentials',{**req,'unexpected':fixture})[0]==422)
 for target in ['openrouter_key','typesafe_key','okx_demo','okx_live']:
  payload={'target':target,'api_key':fixture}
  if target.startswith('okx_'):payload.update(secret='synthetic-secret',passphrase='synthetic-passphrase')
  c,v=request('/api/settings/credentials',payload)
  ok(target+' saves without echo',c==200 and fixture not in json.dumps(v))
  raw=(folder/target).read_bytes()
  ok(target+' encrypted on disk with private mode',raw.startswith(b'OKXS1') and fixture.encode() not in raw and (folder/target).stat().st_mode&0o777==0o600)
 c,s=request('/api/settings');ok('Only configured flags returned',all(x=={'configured':True} for x in s['credentials'].values()) and fixture not in json.dumps(s))
 ok('Credential save does not enable AI',not s['enabled'])
 ok('Explicit budget activation works',request('/api/jev/config',{'budget_usd':'1','enabled':True,'confirmed':True})[0]==200)
 ok('Unknown provider rejected',request('/api/settings/provider',{'provider':'other'})[0]==422)
 ok('Choose direct Jev',request('/api/settings/provider',{'provider':'typesafe'})[0]==200)
 s=request('/api/settings')[1];ok('Provider switch disables AI without resetting costs',s['provider']=='typesafe' and not s['enabled'] and s['budget_usd']=='1' and s['spent_usd']=='0')
 r=request('/api/research')[1];ok('Research follows selected provider',r['jev']['provider']=='typesafe' and r['jev']['model']=='jev-1.13.0' and r['jev']['configured'])
 ok('Activate direct provider explicitly',request('/api/jev/config',{'budget_usd':'1','enabled':True,'confirmed':True})[0]==200)
 ok('Replace AI key',request('/api/settings/credentials',{'target':'typesafe_key','api_key':fixture+'-replacement'})[0]==200)
 ok('Key replacement disables AI',not request('/api/settings')[1]['enabled'])
 b.run('docker','restart',m['container']);b.wait(lambda:request('/api/settings')[0]==200,30)
 s=request('/api/settings')[1];ok('Provider and credential status persist after restart',s['provider']=='typesafe' and all(x['configured'] for x in s['credentials'].values()))
 ok('Delete selected provider key',request('/api/settings/credentials',{'target':'typesafe_key','remove':True})[0]==200)
 ok('Deleted key is absent; other providers intact',not request('/api/settings')[1]['credentials']['typesafe_key']['configured'] and request('/api/settings')[1]['credentials']['openrouter_key']['configured'])
 ok('Deleted selected key prevents enablement',request('/api/jev/config',{'budget_usd':'1','enabled':True,'confirmed':True})[0]==422)
 raw=bytearray((folder/'openrouter_key').read_bytes());raw[-1]^=1;(folder/'openrouter_key').write_bytes(raw)
 ok('Tampered envelope is not configured',not request('/api/settings')[1]['credentials']['openrouter_key']['configured'])
 # Leave all synthetic keys removed for the UI test.
 for target in ['openrouter_key','okx_demo','okx_live']:
  ok('Remove '+target,request('/api/settings/credentials',{'target':target,'remove':True})[0]==200)
 logs=b.run('docker','logs',m['container']).stdout
 ok('Logs do not contain credential input',fixture not in logs)
 ok('No account, orders or provider decisions created',b.state()['account'] is None and not request('/api/research')[1]['decisions'])
 ok('User account unchanged',json.load(urllib.request.urlopen('http://127.0.0.1:4780/api/paper'))['account']==user_before)
 print(str(len(checks))+' checks passed',flush=True)

def cleanup():
 import shutil
 m=json.loads(b.MANIFEST.read_text());b.cleanup()
 d=b.ROOT/'.local'/m['container']
 assert d.name.startswith('okx-phase2-test-')
 shutil.rmtree(d)
if __name__=='__main__':{'provision':b.provision,'check':check,'cleanup':cleanup}[sys.argv[1]]()
