"""Disposable login/session/CSRF test, never changes user's authentication."""
import phase3_acceptance as p
import json,secrets,hashlib,urllib.request,urllib.error,http.cookiejar,time,os
from pathlib import Path
b=p.b;m=json.loads(b.MANIFEST.read_text());name=m['container']+'-auth';path=b.ROOT/'.local'/(name+'.hash');password=secrets.token_urlsafe(24);salt=os.urandom(32);digest=hashlib.pbkdf2_hmac('sha256',password.encode(),salt,600000)
fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
with os.fdopen(fd,'w') as f:f.write(salt.hex()+':'+digest.hex())
checks=[];jar=http.cookiejar.CookieJar();client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar));base='http://127.0.0.1:4782'
def req(route,body=None,origin=True):
 headers={'Host':'127.0.0.1:8080'}
 if body is not None:
  headers.update({'Content-Type':'application/json','X-Terminal-Request':'paper-v2'})
  if origin:headers['Origin']='http://127.0.0.1:8080'
 try:
  r=client.open(urllib.request.Request(base+route,data=None if body is None else json.dumps(body).encode(),headers=headers),timeout=10);return r.status,r.read()
 except urllib.error.HTTPError as e:return e.code,e.read()
def ok(label,condition):
 checks.append({'check':label,'passed':bool(condition)});print(('PASS ' if condition else 'FAIL ')+label,flush=True);assert condition
try:
 b.run('docker','create','--name',name,'--network','okx-tradingterminal_market','--read-only','--memory=512m','--cpus=1','--cap-drop=ALL','-e','EXECUTION_MODE=PAPER_LOCAL','-e','DATABASE_HOST='+b.DB_CONTAINER,'-e','DATABASE_NAME='+m['database'],'-e','DATABASE_PASSWORD_FILE=/run/secrets/db_password','-v',str(b.ROOT/'.local/db_password')+':/run/secrets/db_password:ro','-v',str(path)+':/run/secrets/auth_password:ro','-p','127.0.0.1:4782:8080',b.IMAGE)
 b.run('docker','network','connect','okx-tradingterminal_storage',name);b.run('docker','start',name)
 b.wait(lambda:req('/api/health')[0]==200,20)
 ok('Unauthenticated private API blocked',req('/api/paper')[0]==401)
 ok('Login requires same-origin request',req('/api/auth/login',{'password':password},False)[0]==403)
 ok('Wrong password refused',req('/api/auth/login',{'password':'wrong-test-only'})[0]==401)
 ok('Correct password creates session',req('/api/auth/login',{'password':password})[0]==200 and len(jar)==1)
 cookie=next(iter(jar));ok('Cookie HttpOnly and SameSite Strict',cookie.has_nonstandard_attr('HttpOnly') and cookie.get_nonstandard_attr('SameSite')=='Strict')
 ok('Authenticated API available',req('/api/paper')[0]==200)
 hashed=hashlib.sha256(cookie.value.encode()).hexdigest();stored=b.sql(m['database'],"SELECT token_hash FROM studio.sessions WHERE token_hash='"+hashed+"'").strip()
 ok('Only hash stored in database',stored==hashed and stored!=cookie.value)
 b.sql(m['database'],"UPDATE studio.sessions SET expires_ms=0 WHERE token_hash='"+hashed+"'")
 ok('Expired session refused',req('/api/paper')[0]==401)
 ok('New login after expiry',req('/api/auth/login',{'password':password})[0]==200)
 ok('Logout revokes session',req('/api/auth/logout',{})[0]==200 and req('/api/paper')[0]==401)
 (b.ROOT/'evidence/authentication.json').write_text(json.dumps({'at':time.time(),'scope':'disposable credentials/database only','checks':checks},indent=2))
finally:
 b.run('docker','rm','-f',name,check=False);path.unlink(missing_ok=True)
