from pathlib import Path
import subprocess,time,urllib.request
root=Path(__file__).resolve().parents[1]
args=['rtk','proxy','docker','compose','-f','compose.yaml']
if (root/'.local/auth_password').is_file():args+=['-f','compose.auth.yaml']
subprocess.run(args+['up','-d','terminal','db'],cwd=root,check=True)
for _ in range(120):
    try:
        with urllib.request.urlopen('http://127.0.0.1:4780/api/ready',timeout=2) as r:
            if r.status==200:break
    except Exception:pass
    time.sleep(.5)
else:raise RuntimeError('Terminal nog niet gereed')
subprocess.run(args+['up','-d','--no-deps','orders-demo','orders-live'],cwd=root,check=True)
subprocess.run(args+['restart','orders-demo','orders-live'],cwd=root,check=True)
print('Demo- en Live-aansluitingen gestart. Saldi verschijnen in de terminal; bevestig je eigen account en handelslimieten voor uitvoering.')
