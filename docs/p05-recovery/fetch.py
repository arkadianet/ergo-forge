import concurrent.futures,datetime,hashlib,json,sys,urllib.request,urllib.error
from pathlib import Path
out=Path('docs/p05-recovery');out.mkdir(exist_ok=True)
def get(pair):
 name,url=pair; r={'name':name,'url':url,'retrievedAt':datetime.datetime.now(datetime.timezone.utc).isoformat()}
 try:
  with urllib.request.urlopen(url,timeout=25) as response:
   body=response.read();r.update(status=response.status,finalUrl=response.url);(out/(name+'.fixture')).write_bytes(body);r.update(file=name+'.fixture',sha256=hashlib.sha256(body).hexdigest(),bytes=len(body))
 except Exception as e:r.update(error=str(e))
 return r
jobs=json.loads(sys.argv[1]);rs=list(concurrent.futures.ThreadPoolExecutor(max_workers=4).map(get,jobs))
p=out/'retrievals.json';old=json.loads(p.read_text()) if p.exists() else [];p.write_text(json.dumps(old+rs,indent=2)+'\n');print(json.dumps(rs,indent=2))
