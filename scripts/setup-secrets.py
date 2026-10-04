#!/usr/bin/env python3
"""Interactive local-only secret provisioning. Never puts values in argv, logs or iCloud."""
import argparse,getpass,json,os
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from pathlib import Path
root=Path(__file__).resolve().parents[1]
assert str(root).startswith('/Users/wp/WPOS-local/projects/okx-tradingterminal')
p=argparse.ArgumentParser();p.add_argument('provider',choices=['openrouter','okx-demo']);args=p.parse_args()
secret_dir=root/'.local';secret_dir.mkdir(mode=0o700,exist_ok=True);os.chmod(secret_dir,0o700)
if args.provider=='openrouter':
 value=getpass.getpass('OpenRouter API-sleutel (onzichtbaar): ').strip()
 if not value:raise SystemExit('Geen sleutel opgeslagen.')
 path=secret_dir/'openrouter_key'
else:
 value=json.dumps({key:getpass.getpass(label+' (onzichtbaar): ').strip() for key,label in [('api_key','OKX Demo API-key'),('secret','OKX Demo secret'),('passphrase','OKX Demo passphrase')]})
 if any(not v for v in json.loads(value).values()):raise SystemExit('Onvolledige invoer; niets opgeslagen.')
 path=secret_dir/'okx_demo.json'
key_path=secret_dir/'secrets.key'
if not key_path.exists():
 fd=os.open(key_path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 with os.fdopen(fd,'wb') as f:f.write(os.urandom(32))
os.chmod(key_path,0o600)
purpose='openrouter_key' if args.provider=='openrouter' else 'okx_demo'
nonce=os.urandom(12);envelope=b'OKXS1'+nonce+AESGCM(key_path.read_bytes()).encrypt(nonce,value.encode(),purpose.encode())
tmp=path.with_suffix('.writing');fd=os.open(tmp,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
with os.fdopen(fd,'wb') as f:f.write(envelope);f.flush();os.fsync(f.fileno())
os.replace(tmp,path)
os.chmod(path,0o600)
print('Sleutel AES-GCM-versleuteld uitsluitend lokaal opgeslagen. Waarde wordt niet getoond.')
if args.provider=='openrouter':print('Herstart met: rtk docker compose -f compose.yaml -f compose.jev.yaml up -d terminal\nBevestig daarna het AI-kostenbudget in Nieuws.')
else:print('Demo-sleutel opgeslagen. Onboarding en acceptatie zijn nog vereist voordat orders mogen worden verzonden.')
