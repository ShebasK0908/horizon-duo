import ast, hashlib, io, json, pathlib, struct, subprocess, tokenize
from html.parser import HTMLParser
R=pathlib.Path('/Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web')
T=pathlib.Path('/tmp/horizon-card-review.bHn9GQ')
player='https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492'
image='https://tests-mac-studio.tail935cc1.ts.net:8491/player-card.png'
base=['python3',str(R/'tools/web_card.py')]
results=[]
def run(args):
 p=subprocess.run(base+args,capture_output=True,text=True);return p
p=run(['--help']);assert p.returncode==0 and 'Generate a static Horizon Player Card candidate' in p.stdout
results.append({'help':p.stdout})
class Parser(HTMLParser):
 def __init__(self,s):self.tags=[];super().__init__();self.feed(s)
 def handle_starttag(self,tag,attrs):self.tags.append((tag,dict(attrs)))
def check(s,title,purl,iurl):
 tags=Parser(s).tags
 meta={d.get('name',d.get('property')):d.get('content') for t,d in tags if t=='meta'}
 expected={'twitter:card':'player','twitter:title':title,'twitter:player':purl,'twitter:player:width':'720','twitter:player:height':'720','twitter:image':iurl,'og:title':title,'og:image':iurl,'og:type':'website'}
 for k,v in expected.items():assert meta[k]==v,(k,meta[k],v)
 assert [d['href'] for t,d in tags if t=='a']==[purl]
 assert [d['src'] for t,d in tags if t=='img']==[iurl]
 assert not any(t in ['script','iframe'] for t,d in tags)
 return meta
out=T/'candidate.html'
p=run(['--player-url',player,'--image-url',image,'--output',str(out)])
assert p.returncode==0,p.stderr
results.append({'command':base+['--player-url',player,'--image-url',image,'--output',str(out)],'exit':p.returncode,'stdout':p.stdout,'metadata':check(out.read_text(),'Horizon multiplayer',player,image)})
title='A <script>alert("x")</script> & \'quoted\''
variant=player+'&title=%22%3E%3Cscript%3E&raw="<>&\''
p=run(['--player-url',variant,'--image-url',image+'?q="<>&\'','--title',title,'--output',str(T/'nested/escaped.html')])
assert p.returncode==0,p.stderr
check((T/'nested/escaped.html').read_text(),title,variant,image+'?q="<>&\'')
results.append({'escaping':'PASS: decoded exact title, player and image; no script/iframe element'})
invalid=['http://example.com/','javascript:alert(1)','//example.com/','https:///path','https://user:pass@example.com/','https://@example.com/','https://:pass@example.com/','https://example.com:bad/','https://example.com:65536/','https://[invalid/','https://example.com/a b','https://example.com/\npath','https://example.com/\tpath']
for flag in ['--player-url','--image-url']:
 for i,value in enumerate(invalid):
  dest=T/f'invalid-{flag[2:]}-{i}.html'
  args=['--player-url',player,'--image-url',image,'--output',str(dest)]
  args[args.index(flag)+1]=value
  p=run(args)
  assert p.returncode==2 and 'Use an absolute HTTPS URL without credentials or whitespace' in p.stderr and 'Traceback' not in p.stderr and not dest.exists(),(value,p.returncode,p.stderr)
results.append({'invalid_urls':len(invalid)*2,'exit':2,'output_created':False})
p=run(['--output',str(T/'missing.html')]);assert p.returncode==2 and not (T/'missing.html').exists()
for ua,name in [('Twitterbot/1.0','twitterbot'),('Mozilla/5.0','browser')]:
 p=subprocess.run(['curl','--silent','--show-error','--max-time','20','-A',ua,'--dump-header',str(T/f'{name}.headers'),'--output',str(T/f'{name}.html'), 'https://tests-mac-studio.tail935cc1.ts.net:8491/share.html'],capture_output=True,text=True)
 assert p.returncode==0,p.stderr
 headers=(T/f'{name}.headers').read_text();data=(T/f'{name}.html').read_bytes()
 assert '200' in headers.splitlines()[0] and data==out.read_bytes()
 check(data.decode(),'Horizon multiplayer',player,image)
 results.append({'ua':ua,'headers':headers,'sha256':hashlib.sha256(data).hexdigest(),'matches_generated':True})
p=subprocess.run(['curl','--silent','--show-error','--max-time','20','--dump-header',str(T/'image.headers'),'--output',str(T/'player-card.png'),image],capture_output=True,text=True);assert p.returncode==0,p.stderr
data=(T/'player-card.png').read_bytes();assert data== (R/'web/player-card.png').read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n';assert struct.unpack('>II',data[16:24])==(1200,630)
results.append({'png_headers':(T/'image.headers').read_text(),'png_bytes':len(data),'dimensions':[1200,630],'sha256':hashlib.sha256(data).hexdigest(),'matches_source':True})
src=(R/'tools/web_card.py').read_text();tree=ast.parse(src)
docs=[n for n in ast.walk(tree) if isinstance(n,ast.Expr) and isinstance(n.value,ast.Constant) and isinstance(n.value.value,str)]
comments=[t.string for t in tokenize.generate_tokens(io.StringIO(src).readline) if t.type==tokenize.COMMENT]
assert len(docs)==1 and 'description=__doc__' in src and comments==['#!/usr/bin/env python3']
results.append({'docstrings':len(docs),'help_uses':'__doc__','comments':comments,'suppressions':[]})
(T/'results.json').write_text(json.dumps(results,indent=2));print(json.dumps(results,indent=2))
