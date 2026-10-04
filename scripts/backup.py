"""AES-GCM backups of PostgreSQL plus immutable Parquet archives. Disposable restore tests."""
import argparse,os,json,subprocess,secrets,time,hashlib,io,tarfile,zipfile
from pathlib import Path
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
ROOT=Path(__file__).resolve().parents[1];LOCAL=ROOT/'.local';BACKUPS=LOCAL/'backups';KEY=LOCAL/'backup.key';DB='okx-tradingterminal-db-1';APP='okx-tradingterminal-terminal-1';MAX_BYTES=256*1024*1024
IMAGE='okx-tradingterminal:0.4.0';DATABASE='terminal'
def run(*args,input=None):
 p=subprocess.run(['rtk','proxy',*args],input=input,stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=ROOT)
 if p.returncode:raise RuntimeError('Opdracht mislukt: '+p.stderr.decode(errors='replace')[:300])
 return p.stdout
def key(create=False):
 LOCAL.mkdir(mode=0o700,exist_ok=True);os.chmod(LOCAL,0o700)
 if create and not KEY.exists():
  fd=os.open(KEY,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
  with os.fdopen(fd,'wb') as f:f.write(os.urandom(32))
 assert KEY.is_file(),'Back-upsleutel ontbreekt; herstel de apart bewaarde sleutel'
 os.chmod(KEY,0o600);return KEY.read_bytes()
def sha(data):return hashlib.sha256(data).hexdigest()
def backup():
 # Dump first, then archives. Immutable files are installed before their manifest row commits.
 dump=run('docker','exec',DB,'pg_dump','-U','terminal','-d',DATABASE,'-Fc')
 archive=run('docker','exec',APP,'tar','cf','-','-C','/archives','.')
 assert len(dump)+len(archive)<=MAX_BYTES,'Back-up groter dan 256 MiB; configureer streaming-backup voor verdere groei'
 files={}
 with tarfile.open(fileobj=io.BytesIO(archive)) as t:
  for member in t:
   name=Path(member.name).name
   if member.isfile() and name.endswith('.parquet'):
    assert '/' not in name and '..' not in name
    data=t.extractfile(member).read();assert data[:4]==b'PAR1' and data[-4:]==b'PAR1'
    files[name]=data
 buf=io.BytesIO()
 manifest={'version':2,'created_ms':int(time.time()*1000),'database_sha256':sha(dump),'archives':{n:sha(v) for n,v in files.items()},'scope':'Database + immutable Parquet; credentials/key excluded'}
 with zipfile.ZipFile(buf,'w',zipfile.ZIP_STORED) as z:
  z.writestr('database.dump',dump);z.writestr('manifest.json',json.dumps(manifest))
  for name,data in files.items():z.writestr('archives/'+name,data)
 nonce=os.urandom(12);payload=b'OKXB2'+nonce+AESGCM(key(True)).encrypt(nonce,buf.getvalue(),b'okx-terminal-backup-v2')
 BACKUPS.mkdir(mode=0o700,exist_ok=True);os.chmod(BACKUPS,0o700)
 path=BACKUPS/(time.strftime('%Y%m%dT%H%M%S')+'-'+secrets.token_hex(3)+'.aesgcm')
 fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 with os.fdopen(fd,'wb') as f:f.write(payload);f.flush();os.fsync(f.fileno())
 print(json.dumps({'backup':str(path),'encrypted_bytes':len(payload),'sha256':sha(payload),'archives':len(files),'key':'separate local .local/backup.key; not included'}))
 return path
def verify(path):
 p=Path(path).resolve();assert p.parent==BACKUPS.resolve(),'Only local backup directory'
 encrypted=p.read_bytes();assert encrypted[:5]==b'OKXB2'
 data=AESGCM(key()).decrypt(encrypted[5:17],encrypted[17:],b'okx-terminal-backup-v2')
 assert len(data)<=MAX_BYTES+1024*1024
 with zipfile.ZipFile(io.BytesIO(data)) as z:
  manifest=json.loads(z.read('manifest.json'));dump=z.read('database.dump');assert sha(dump)==manifest['database_sha256']
  files={}
  for name,digest in manifest['archives'].items():
   assert '/' not in name and '..' not in name and name.endswith('.parquet')
   files[name]=z.read('archives/'+name);assert sha(files[name])==digest
 db='okx_restore_test_'+secrets.token_hex(6);volume=db+'-archives'
 run('docker','exec',DB,'createdb','-U','terminal',db)
 try:
  run('docker','exec','-i',DB,'pg_restore','-U','terminal','-d',db,'--no-owner','--exit-on-error',input=dump)
  def sql(q):return run('docker','exec',DB,'psql','-U','terminal','-d',db,'-At','-c',q).decode().strip()
  expected=sql('SELECT path FROM studio.history_parts').splitlines()
  assert all(name in files for name in expected),'Manifest verwijst naar ontbrekend archief'
  buf=io.BytesIO()
  with tarfile.open(fileobj=buf,mode='w') as t:
   for name,value in files.items():
    info=tarfile.TarInfo(name);info.size=len(value);info.mode=0o600;t.addfile(info,io.BytesIO(value))
  run('docker','volume','create',volume)
  run('docker','run','--rm','-i','--network','none','--read-only','--cap-drop=ALL','-v',volume+':/archives','--entrypoint','tar',IMAGE,'xf','-','-C','/archives',input=buf.getvalue())
  restored=run('docker','run','--rm','--network','none','--read-only','--cap-drop=ALL','-v',volume+':/archives:ro','--entrypoint','tar',IMAGE,'cf','-','-C','/archives','.')
  with tarfile.open(fileobj=io.BytesIO(restored)) as t:
   actual={Path(m.name).name:sha(t.extractfile(m).read()) for m in t if m.isfile()}
  assert actual==manifest['archives']
  result={'restore_verified':True,'scope':'temporary isolated database and archive volume','source_database':DATABASE,'paper_accounts':int(sql('SELECT count(*) FROM paper_v2.account')),'tables':int(sql("SELECT count(*) FROM information_schema.tables WHERE table_schema IN ('paper_v2','studio')")),'archive_files':len(actual),'backup':str(p),'at':time.time()}
  (ROOT/'evidence/restore.json').write_text(json.dumps(result,indent=2));print(json.dumps(result))
 finally:
  run('docker','exec',DB,'dropdb','-U','terminal',db);run('docker','volume','rm',volume)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('action',choices=['backup','verify','test']);p.add_argument('file',nargs='?');args=p.parse_args()
 if args.action=='backup':backup()
 elif args.action=='test':verify(backup())
 else:verify(args.file)
