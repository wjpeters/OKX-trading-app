#!/usr/bin/env python3
import getpass,hashlib,os
from pathlib import Path
root=Path(__file__).resolve().parents[1]
a=getpass.getpass('Nieuw lokaal wachtwoord (minimaal 14 tekens): ')
b=getpass.getpass('Herhaal: ')
if a!=b or len(a)<14:raise SystemExit('Wachtwoorden verschillen of zijn te kort. Niets opgeslagen.')
salt=os.urandom(32);digest=hashlib.pbkdf2_hmac('sha256',a.encode(),salt,600000)
d=root/'.local';d.mkdir(mode=0o700,exist_ok=True);os.chmod(d,0o700)
f=os.open(d/'auth_password',os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
with os.fdopen(f,'w') as out:out.write(salt.hex()+':'+digest.hex()+'\n')
print('Alleen gezouten wachtwoordhash lokaal opgeslagen. Herstart met compose.auth.yaml. Sessies verlopen na 8 uur. De app blijft uitsluitend op localhost bereikbaar.')
