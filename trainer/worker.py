"""Credential-free CMA-ES worker. Durable generation checkpoints; no order API.
The Rust child is the sole strategy/evaluation implementation.
"""
import os,json,time,hashlib,pickle,threading,concurrent.futures,subprocess,math,statistics,fcntl,resource,selectors
from pathlib import Path
from datetime import datetime
from zoneinfo import ZoneInfo
import numpy as np
import cma

ROOT=Path(os.environ.get('TRAINING_DIR','/training'))
DAY=86_400_000
PIN='7b928d828b7b0e022f929d9bd2e44165aa270148'
WEIGHTS=340
FEATURES='causal-ohlcv-v1'
PARAMETERS=[('fast',2,30,True),('slow',31,100,True),('lookback',10,100,True),('rsi_low',10,45,False),('bb_z',1,3,False),('volume_ratio',.5,3,False),('min_atr',0,.02,False),('stop_bps',25,300,False),('target_bps',30,600,False),('max_hold',2,288,True),('cooldown',0,100,True),('confidence',.25,.8,False)]
STOP=threading.Event()

def atomic(path,value,raw=False):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    data=value if raw else json.dumps(value,separators=(',',':'),allow_nan=False).encode()
    tmp=path.with_suffix('.tmp')
    with open(tmp,'wb') as f:f.write(data);f.flush();os.fsync(f.fileno())
    os.replace(tmp,path)
def read(path,default=None):
    try:return json.loads(Path(path).read_text())
    except FileNotFoundError:return default
def digest(path):
    h=hashlib.sha256()
    with open(path,'rb') as f:
        while b:=f.read(1024*1024):h.update(b)
    return h.hexdigest()
def current_day():return datetime.now(ZoneInfo('Europe/Amsterdam')).date().isoformat()
class Budget:
    def __init__(self):self.path=ROOT/'resource-budget.json';self.value=read(self.path,{'day':current_day(),'seconds':0});self.last=time.monotonic();self.lock=threading.Lock()
    def tick(self,active):
        with self.lock:
            now=time.monotonic()
            if self.value['day']!=current_day():self.value={'day':current_day(),'seconds':0}
            if active:self.value['seconds']+=now-self.last
            self.last=now;atomic(self.path,self.value)
    def available(self,hours):return self.value['seconds']<hours*3600
BUDGET=None

class Evaluator:
    def __init__(self,path,bar,laya_path=None):
        env={'PATH':os.environ.get('PATH',''),'TZ':'UTC'}
        self.p=subprocess.Popen(['/usr/bin/nice','-n','19',os.environ.get('EVALUATOR','/app/okx-evaluator')],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,bufsize=1,env=env)
        self.meta=self.call({'command':'load','path':path,'bar':bar,**({'laya_path':str(laya_path)} if laya_path else {})})
    def call(self,req):
        self.p.stdin.write(json.dumps(req,separators=(',',':'),allow_nan=False)+'\n');self.p.stdin.flush()
        selector=selectors.DefaultSelector();selector.register(self.p.stdout,selectors.EVENT_READ)
        while not selector.select(.5):
            limit=read(ROOT/'active.json',{}).get('daily_hours',8)
            if BUDGET and not BUDGET.available(limit):self.p.terminate();selector.close();raise Pause()
        selector.close();line=self.p.stdout.readline()
        if not line:raise RuntimeError('Rust-evaluator gestopt (resourcegrens of procesfout)')
        out=json.loads(line)
        if 'error'in out:raise RuntimeError(out['error'])
        return out
    def close(self):
        self.p.terminate()
        try:self.p.wait(timeout=3)
        except subprocess.TimeoutExpired:self.p.kill();self.p.wait()
    def __enter__(self):return self
    def __exit__(self,*_):self.close()

def parameter_vector(x):
    result={}
    for v,(name,lo,hi,integer) in zip(x[WEIGHTS:],PARAMETERS):
        bounded=max(-12,min(12,float(v)));value=lo+(hi-lo)/(1+math.exp(-bounded))
        result[name]=round(value) if integer else value
    return result
def artifact(x,norm,ds,bar,config,seed,generation):
    return {'schema':1,'feature_version':FEATURES,'weights':[float(w) for w in x[:WEIGHTS]],'parameters':parameter_vector(x),'normalizer':norm,'laya':config['laya'],'laya_checkpoint':PIN if config['laya'] else '','laya_runtime':config.get('laya_runtime',''), 'instrument':ds['instrument'],'bar':bar,'dataset_hash':ds['hash'],'seed':int(seed),'generation':int(generation)}
def trim(report):return {k:v for k,v in report.items() if k not in ('curve','decisions','daily_returns','activation')}
def split_windows(ds):
    dev_end=ds['holdout_start_ms'];windows=[]
    for i in range(4):
        train_start=dev_end-(300-i*30)*DAY
        train_end=train_start+180*DAY
        windows.append({'train':[train_start,train_end],'validation':[train_end,train_end+30*DAY]})
    if windows[0]['train'][0]<ds['start_ms']:raise ValueError('Twaalf maanden historie ontbreekt')
    return windows
def train_blocks(start,end):
    span=(end-start)//4
    return [(start+i*span,end if i==3 else start+(i+1)*span)for i in range(4)]
def score(reports):
    if not reports or any(not r['valid'] for r in reports):return 1e6
    # Round net return to 1bp; lower drawdown resolves ties, not trade count/win rate.
    return -statistics.median(r['net_return']for r in reports)+max(r['max_drawdown']for r in reports)*1e-12
def evaluate(e,a,ds,start,end,mode='neural',stress=1):
    return e.call({'command':'evaluate','artifact':a,'costs':{**ds['costs'],'stress':str(stress)},'start_ms':start,'end_ms':end,'mode':mode})
def laya_request(rows,ds,bar,runtime):
    from urllib.request import Request,urlopen
    req=Request(os.environ.get('LAYA_URL','http://host.docker.internal:8092')+'/regime',data=json.dumps({'rows':rows,'instrument':ds['instrument'],'bar':bar,'checkpoint':PIN}).encode(),headers={'Content-Type':'application/json'})
    with urlopen(req,timeout=120)as r:value=json.load(r)
    if value.get('checkpoint')!=PIN or value.get('runtime')!=runtime:raise RuntimeError('Laya-checkpoint wijkt af')
    return value['rows']
class Pause(Exception):pass
def boundary(config,progress,folder):
    desired=read(folder/'control.json',{}).get('desired','PAUSED')
    if desired!='RUNNING':progress['status']=desired;atomic(folder/'progress.json',progress);raise Pause()
    if not BUDGET.available(config['daily_hours']):progress['status']='DAILY_BUDGET';atomic(folder/'progress.json',progress);raise Pause()
def cache_laya(config,ds,bar,folder,progress):
    path=folder/f'laya-{ds["id"]}-{bar}.json'
    if not config['laya']:return None
    cache=read(path,[]);after=cache[-1]['ts']+1 if cache else ds['start_ms']
    with Evaluator(ds['path'],bar)as e:
        while after<ds['holdout_start_ms']:
            boundary(config,progress,folder)
            rows=e.call({'command':'features','after':after,'end_ms':ds['holdout_start_ms'],'limit':8})
            if not rows:break
            values=laya_request(rows,ds,bar,config['laya_runtime']);cache.extend(values);after=rows[-1]['ts']+1
            progress.update(status='LAYA_FEATURES',feature_rows=len(cache),feature_bar=bar,market=ds['instrument']);atomic(path,cache);atomic(folder/'progress.json',progress)
    return path
def checkpoint(path,state):atomic(path,pickle.dumps(state,protocol=5),raw=True)
def optimize(config,ds,bar,folder,progress,job,start,end,norm,laya_path,initial=None):
    cp=folder/f'{job}.checkpoint';saved=pickle.loads(cp.read_bytes())if cp.exists()else None
    finalists=saved.get('finalists',[])if saved else [];seed_index=saved.get('seed_index',0)if saved else 0
    es=saved.get('es')if saved else None;best=saved.get('best')if saved else None;stale=saved.get('stale',0)if saved else 0
    if saved and 'rng' in saved:np.random.set_state(saved['rng'])
    evaluators=[Evaluator(ds['path'],bar,laya_path)for _ in range(config['workers'])]
    available=list(evaluators);condition=threading.Condition()
    def evaluate_x(x):
        with condition:
            while not available:condition.wait()
            ev=available.pop()
        try:
            art=artifact(x,norm,ds,bar,config,config['seeds'][seed_index],es.countiter+1)
            rs=[evaluate(ev,art,ds,s,t)for s,t in train_blocks(start,end)]
            return score(rs),art,rs
        finally:
            with condition:available.append(ev);condition.notify()
    try:
        with concurrent.futures.ThreadPoolExecutor(max_workers=config['workers'])as pool:
            while seed_index<len(config['seeds']):
                seed=config['seeds'][seed_index]
                if es is None:
                    x0=np.array(initial if initial is not None else [0.]*(WEIGHTS+len(PARAMETERS)))
                    es=cma.CMAEvolutionStrategy(x0,.35,{'popsize':config['population'],'seed':seed,'bounds':[-8,8],'maxiter':config['generations'],'verb_disp':0,'verb_log':0,'CMA_diagonal':20,'tolfun':0,'tolflatfitness':config['patience'],'maxfevals':config['population']*config['generations']})
                    best=None;stale=0
                while es.countiter<config['generations']and stale<config['patience']and not es.stop():
                    boundary(config,progress,folder)
                    xs=es.ask();t=time.monotonic();results=list(pool.map(evaluate_x,xs));es.tell(xs,[r[0]for r in results]);elapsed=time.monotonic()-t;BUDGET.tick(True)
                    chosen=min(results,key=lambda r:r[0]);improved=best is None or chosen[0]<best[0]-1e-9
                    if improved:best=(chosen[0],xs[results.index(chosen)].tolist(),chosen[1],[trim(r)for r in chosen[2]]);stale=0
                    else:stale+=1
                    progress.update(status='TRAINING',market=ds['instrument'],bar=bar,phase=job,seed=seed,generation=es.countiter,max_generations=config['generations'],evaluated=progress.get('evaluated',0)+len(xs),compute_seconds=progress.get('compute_seconds',0)+elapsed,best_score=best[0],best_artifact=best[2],last_reports=best[3],day_seconds=BUDGET.value['seconds'])
                    history=progress.setdefault('history',[]);history.append({'generation':len(history)+1,'seed':seed,'phase':job,'market':ds['instrument'],'bar':bar,'fitness':best[0],'net_return':statistics.median(r['net_return']for r in best[3]),'drawdown':max(r['max_drawdown']for r in best[3]),'seconds':elapsed,'snapshot':f'{job}-{seed}-{es.countiter}'})
                    # A durable optimizer + RNG state only after a complete generation; interrupted generation repeats deterministically.
                    checkpoint(cp,{'seed_index':seed_index,'es':es,'best':best,'stale':stale,'finalists':finalists,'rng':np.random.get_state()})
                    atomic(folder/'progress.json',progress)
                    atomic(folder/'generations'/f'{job}-{seed}-{es.countiter}.json',{'artifact':best[2],'metrics':history[-1]})
                if best is not None:finalists.append(best)
                seed_index+=1;es=None;best=None;stale=0;checkpoint(cp,{'seed_index':seed_index,'es':None,'best':None,'stale':0,'finalists':finalists})
        return finalists
    finally:
        for e in evaluators:e.close()
def campaign(folder):
    config=read(folder/'request.json');progress=read(folder/'progress.json',{'status':'QUEUED','artifacts':[],'history':[],'compute_seconds':0})
    if progress.get('status')in ('COMPLETED','NO_SUITABLE_BOT','WAITING_NEW_DATA','STOPPED'):return
    boundary(config,progress,folder)
    for ds in config['datasets']:
        if digest(ds['path'])!=ds['hash']:raise RuntimeError('Dataset is gewijzigd na vastzetten')
        windows=split_windows(ds)
        for bar in config['bars']:
            if any(x['summary']['instrument']==ds['instrument']and x['summary']['bar']==bar for x in progress['artifacts']):continue
            laya_path=cache_laya(config,ds,bar,folder,progress)
            with Evaluator(ds['path'],bar,laya_path)as e:
                finalists=[]
                for wi,win in enumerate(windows):
                    norm=e.call({'command':'fit','start_ms':win['train'][0],'end_ms':win['train'][1]})
                    fold=optimize(config,ds,bar,folder,progress,f'fold{wi}-{ds["id"]}-{bar}',*win['train'],norm,laya_path)
                    for fitness,x,art,training_reports in fold:
                        rs=evaluate(e,art,ds,*win['validation'])
                        finalists.append({'seed':art['seed'],'x':x,'report':rs,'fold':wi})
                if not finalists:progress['status']='NO_SUITABLE_BOT';atomic(folder/'progress.json',progress);continue
                validation=[]
                for seed in config['seeds']:
                    # Never score a genotype on an earlier validation window than its own training.
                    group=sorted([r for r in finalists if r['seed']==seed],key=lambda r:r['fold'])
                    if len(group)==4:validation.append((score([r['report']for r in group]),group[-1]['x'],[r['report']for r in group]))
                if not validation:raise RuntimeError('Vier onafhankelijke walk-forwardrondes ontbreken')
                chosen=min(validation,key=lambda x:x[0]);dev_start=ds['start_ms'];dev_end=ds['holdout_start_ms']
                norm=e.call({'command':'fit','start_ms':dev_start,'end_ms':dev_end})
                retrained=optimize(config,ds,bar,folder,progress,f'refit-{ds["id"]}-{bar}',dev_start,dev_end,norm,laya_path,chosen[1])
                if not retrained:continue
                best=min(retrained,key=lambda x:x[0]);art=best[2];art['normalizer']=norm
                report=evaluate(e,art,ds,dev_start,dev_end)
                benchmarks={mode:trim(evaluate(e,art,ds,dev_start,dev_end,mode))for mode in ('trend','breakout','mean_reversion','cash')}
                no_laya={**art,'laya':False,'laya_checkpoint':''};benchmarks['without_laya']=trim(evaluate(e,no_laya,ds,dev_start,dev_end))
                # Buy-and-hold uses the same Rust strategy/fill cost engine through a dedicated mode.
                benchmarks['buy_and_hold']=trim(evaluate(e,art,ds,dev_start,dev_end,'buy_and_hold'))
                payload=json.dumps(art,separators=(',',':'),allow_nan=False).encode();h=hashlib.sha256(payload).hexdigest();atomic(folder/'artifacts'/f'{h}.json',payload,raw=True)
                summary={'instrument':ds['instrument'],'bar':bar,'validation':[trim(r)for r in chosen[2]],'development':trim(report),'benchmarks':benchmarks,'activation':report['activation'],'curve':report['curve'],'windows':windows,'holdout_used':False,'laya_feature_hash':digest(laya_path)if laya_path else None,'local_compute_seconds':progress['compute_seconds'],'search_exhausted':True,'selection':'median net training return, drawdown tie-break; validation selection; frozen refit'}
                progress['artifacts'].append({'hash':h,'summary':summary});atomic(folder/'progress.json',progress)
    progress['status']='WAITING_NEW_DATA'if progress['artifacts']else'NO_SUITABLE_BOT';atomic(folder/'progress.json',progress)

def bootstrap_p(daily,seed,reps=2000):
    values=np.array([x['return']for x in daily],dtype=float)
    if len(values)<30:return 1.,None
    # Circular day blocks preserve autocorrelation. Null-centered one-sided mean test.
    rng=np.random.default_rng(seed);length=max(2,round(len(values)**(1/3)));mean=float(values.mean());centered=values-mean;means=[];null=[]
    for _ in range(reps):
        starts=rng.integers(0,len(values),math.ceil(len(values)/length));idx=np.concatenate([(s+np.arange(length))%len(values)for s in starts])[:len(values)]
        means.append(float(values[idx].mean()));null.append(float(centered[idx].mean()))
    return (1+sum(v>=mean for v in null))/(reps+1),[float(np.quantile(means,.025)),float(np.quantile(means,.975))]
def holm(ps,alpha=.05):
    order=sorted(range(len(ps)),key=lambda i:ps[i]);passed=[False]*len(ps);proceed=True
    for rank,i in enumerate(order):proceed=proceed and ps[i]<=alpha/(len(ps)-rank);passed[i]=proceed
    return passed
def qualify(folder):
    req=read(folder/'request.json');art=req['artifact'];config=req['config'];ds=next(d for d in config['datasets']if d['instrument']==art['instrument']);camp=ROOT/'campaigns'/config['id'];path=camp/f'laya-{ds["id"]}-{art["bar"]}-holdout.json'
    if config['laya']:
        cache=read(path,[]);after=cache[-1]['ts']+1 if cache else ds['holdout_start_ms']
        with Evaluator(ds['path'],art['bar'])as e:
            while True:
                if not BUDGET.available(config['daily_hours']):raise Pause()
                rows=e.call({'command':'features','after':after,'end_ms':ds['end_ms'],'limit':8})
                if not rows:break
                cache.extend(laya_request(rows,ds,art['bar'],art['laya_runtime']));after=rows[-1]['ts']+1;atomic(path,cache)
    with Evaluator(ds['path'],art['bar'],path if config['laya']else None)as e:
        report=evaluate(e,art,ds,ds['holdout_start_ms'],ds['end_ms']);stress=evaluate(e,art,ds,ds['holdout_start_ms'],ds['end_ms'],stress=1.5)
        p,ci=bootstrap_p(report['daily_returns'],config['qualification']['bootstrap_seed'])
        # Bonferroni bound at first submission is valid for the fixed complete qualification family.
        # Full Holm step-down is recomputed when the campaign's reports all exist.
        family=len(config['datasets'])*len(config['bars']);statistical=p<=.05/family and ci is not None and ci[0]>0
        checks={'positive_net':report['net_return']>0,'profit_factor':(report['profit_factor']or 0)>=1.2,'roundtrips':report['roundtrips']>=200,'drawdown':report['max_drawdown']<=.05 and report['valid'],'cost_stress':stress['net_return']>0 and stress['valid'],'statistics':statistical}
        benchmarks={mode:trim(evaluate(e,art,ds,ds['holdout_start_ms'],ds['end_ms'],mode))for mode in ('trend','breakout','mean_reversion','cash','buy_and_hold')};benchmarks['without_laya']=trim(evaluate(e,{**art,'laya':False,'laya_checkpoint':''},ds,ds['holdout_start_ms'],ds['end_ms']))
        atomic(folder/'report.json',{'passed':all(checks.values()),'checks':checks,'historical':report,'stress':trim(stress),'benchmarks':benchmarks,'bootstrap_p':p,'bootstrap_ci':ci,'family_size':family,'family_complete':False,'correction':'Pending Holm family completion','holdout_consumed':True,'reason':'geslaagd'if all(checks.values())else'onvoldoende bewijs / afgewezen','forward_required':True,'demo_required':True,'hash':req['hash']})

def finalize_families():
    for camp in (ROOT/'campaigns').glob('*'):
        config=read(camp/'request.json',{});p=read(camp/'progress.json',{});artifacts=p.get('artifacts',[])
        expected=len(config.get('datasets',[]))*len(config.get('bars',[]))
        if not expected or len(artifacts)!=expected:continue
        paths=[ROOT/'qualification'/a['hash']/'report.json'for a in artifacts];reports=[read(path)for path in paths]
        if any(r is None for r in reports)or all(r.get('family_complete')for r in reports):continue
        accepted=holm([r.get('bootstrap_p',1.)for r in reports])
        for path,report,stat in zip(paths,reports,accepted):
            ci=report.get('bootstrap_ci');report.setdefault('checks',{})['statistics']=bool(stat and ci and ci[0]>0)
            report.update(family_complete=True,correction='Holm step-down over all predeclared campaign candidates',passed=all(report['checks'].values()))
            report['reason']='geslaagd'if report['passed']else'onvoldoende bewijs / afgewezen'
            atomic(path,report)

def heartbeat():
    while not STOP.wait(5):
        active=read(ROOT/'active.json',{});BUDGET.tick(bool(active.get('active')))
        atomic(ROOT/'heartbeat.json',{'status':'online','at_ms':int(time.time()*1000),'seconds_today':BUDGET.value['seconds'],'memory_mb':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024,'pid':os.getpid(),'evaluator':'shared Rust kernel','credentials':False})
def main():
    global BUDGET
    ROOT.mkdir(parents=True,exist_ok=True);lock=open(ROOT/'worker.lock','a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    BUDGET=Budget();threading.Thread(target=heartbeat,daemon=True).start()
    while True:
        active=False
        for folder in sorted((ROOT/'qualification').glob('*'))if (ROOT/'qualification').exists()else []:
            if not(folder/'request.json').exists()or(folder/'report.json').exists():continue
            try:
                req=read(folder/'request.json')
                if not BUDGET.available(req['config']['daily_hours']):continue
                atomic(ROOT/'active.json',{'active':True,'daily_hours':req['config']['daily_hours']});qualify(folder)
            except Pause:pass
            except Exception as e:atomic(folder/'report.json',{'passed':False,'checks':{'technical':False},'reason':str(e),'holdout_consumed':True})
            finally:atomic(ROOT/'active.json',{'active':False});BUDGET.tick(False)
        finalize_families()
        for folder in sorted((ROOT/'campaigns').glob('*'))if (ROOT/'campaigns').exists()else []:
            if not(folder/'request.json').exists():continue
            config=read(folder/'request.json');atomic(ROOT/'active.json',{'active':True,'daily_hours':config['daily_hours']});active=True
            try:campaign(folder)
            except Pause:pass
            except Exception as e:
                p=read(folder/'progress.json',{});p.update(status='ERROR',error=str(e));atomic(folder/'progress.json',p)
            finally:atomic(ROOT/'active.json',{'active':False});BUDGET.tick(False)
        time.sleep(3)
if __name__=='__main__':main()
