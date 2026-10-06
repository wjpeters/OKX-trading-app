from pathlib import Path
import secrets, os
root=Path(__file__).resolve().parents[1]
d=root/'.local';d.mkdir(mode=0o700,exist_ok=True);d.chmod(0o700)
f=d/'db_password'
if not f.exists():
 fd=os.open(f,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 with os.fdopen(fd,'w') as stream:stream.write(secrets.token_hex(32))
f.chmod(0o600)
print('Lokale database-secret gereed. Inhoud wordt niet getoond.')

# Migrate existing encrypted envelopes once; never read or print plaintext credentials.
import shutil
key=d/'secrets.key'
if not key.exists():
 if any((d/name).exists() for name in ['openrouter_key','okx_demo.json']) or any((d/'provider-settings'/name).exists() for name in ['openrouter_key','typesafe_key','okx_demo','okx_live']):
  raise SystemExit('Bestaande versleutelde keys gevonden zonder hoofdsleutel. Herstel secrets.key; deze wordt niet vervangen.')
 fd=os.open(key,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 with os.fdopen(fd,'wb') as stream:stream.write(secrets.token_bytes(32))
key.chmod(0o600)
settings=d/'provider-settings';settings.mkdir(mode=0o700,exist_ok=True);settings.chmod(0o700)
marker=settings/'.migrated-v1'
if not marker.exists():
 for old,new in [('openrouter_key','openrouter_key'),('okx_demo.json','okx_demo')]:
  source=d/old;target=settings/new
  if source.is_file() and not target.exists():
   shutil.copyfile(source,target);target.chmod(0o600)
 marker.touch(mode=0o600)
print('Lokale versleutelde provideropslag gereed. Sleutels worden niet getoond.')
