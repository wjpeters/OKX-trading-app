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
