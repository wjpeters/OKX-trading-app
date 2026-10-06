#!/usr/bin/env python3
"""Interactive local-only secret provisioning. Never puts values in argv, logs or iCloud."""
import argparse,getpass,json,os,subprocess
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from pathlib import Path
root=Path(__file__).resolve().parents[1]
assert str(root).startswith('/Users/wp/WPOS-local/projects/okx-tradingterminal')
running=subprocess.run(['rtk','proxy','docker','compose','ps','--status','running','-q','terminal'],cwd=root,capture_output=True,text=True)
if running.returncode != 0 or running.stdout.strip():
 raise SystemExit('Gebruik Systeem in de actieve terminal voor sleutelbeheer, of stop de terminal voor deze offline setup.')
p=argparse.ArgumentParser();p.add_argument('provider',choices=['openrouter','typesafe','okx-demo','okx-live']);args=p.parse_args()
subprocess.run(['rtk','proxy','python3',str(root/'scripts/bootstrap.py')],check=True)
secret_dir=root/'.local/provider-settings'
if args.provider in ['openrouter','typesafe']:
 value=getpass.getpass(args.provider+' API-sleutel (onzichtbaar): ').strip()
 if not value:raise SystemExit('Geen sleutel opgeslagen.')
 path=secret_dir/('openrouter_key' if args.provider=='openrouter' else 'typesafe_key')
else:
 value=json.dumps({key:getpass.getpass(label+' (onzichtbaar): ').strip() for key,label in [('api_key',args.provider+' API-key'),('secret',args.provider+' secret'),('passphrase',args.provider+' passphrase')]})
 if any(not v for v in json.loads(value).values()):raise SystemExit('Onvolledige invoer; niets opgeslagen.')
 path=secret_dir/('okx_demo' if args.provider=='okx-demo' else 'okx_live')
key_path=root/'.local/secrets.key'
if not key_path.exists():
 fd=os.open(key_path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 with os.fdopen(fd,'wb') as f:f.write(os.urandom(32))
os.chmod(key_path,0o600)
purpose=path.name
nonce=os.urandom(12);envelope=b'OKXS1'+nonce+AESGCM(key_path.read_bytes()).encrypt(nonce,value.encode(),purpose.encode())
tmp=path.with_suffix('.writing');fd=os.open(tmp,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
with os.fdopen(fd,'wb') as f:f.write(envelope);f.flush();os.fsync(f.fileno())
os.replace(tmp,path)
os.chmod(path,0o600)
print('Sleutel AES-GCM-versleuteld uitsluitend lokaal opgeslagen. Waarde wordt niet getoond.')
if args.provider in ['openrouter','typesafe']:print('Kies de provider in Systeem en bevestig daarna het AI-kostenbudget. Opslaan via Systeem verdient de voorkeur tijdens gebruik.')
else:print('Demo-sleutel opgeslagen. Onboarding en acceptatie zijn nog vereist voordat orders mogen worden verzonden.')
