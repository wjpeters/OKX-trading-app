from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
subprocess.run(['rtk','proxy','python3','scripts/bootstrap.py'],cwd=root,check=True)
args=['rtk','proxy','docker','compose','-f','compose.yaml']
for secret,overlay in [('openrouter_key','compose.jev.yaml'),('auth_password','compose.auth.yaml')]:
 if (root/'.local'/secret).is_file():args+=['-f',overlay]
subprocess.run(args+['up','-d','--build'],cwd=root,check=True)
