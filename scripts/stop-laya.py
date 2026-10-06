"""Stop only the process recorded for this project's local Laya service."""
from pathlib import Path
import subprocess,os,signal
root=Path(__file__).resolve().parents[1];path=root/'.local/laya.pid'
if path.exists():
 pid=int(path.read_text())
 command=subprocess.run(['rtk','proxy','ps','-o','command=','-p',str(pid)],capture_output=True,text=True).stdout
 if str(root/'trainer/laya_service.py')in command:os.kill(pid,signal.SIGTERM);print('Lokale Laya-service gestopt')
 path.unlink()
