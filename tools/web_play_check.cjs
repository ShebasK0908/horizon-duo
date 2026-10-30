const {chromium}=require(process.env.HORIZON_PLAYWRIGHT || 'playwright');
const fs=require('node:fs');
const output=process.env.HORIZON_WEB_PROOF || '.local/web-play-proof';
const bundle=process.env.HORIZON_WEB_BUNDLE || '.local/web-owned/play-resources.zip';
const base=process.env.HORIZON_WEB_URL || 'http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:8767';
async function main() {
 fs.mkdirSync(output,{recursive:true});
 const browser=await chromium.launch({channel:'chrome',headless:true,args:['--enable-unsafe-webgpu']});
 const logs=[]; const evidence={passed:false,base};
 async function join(name) {
  const page=await browser.newPage({viewport:{width:1000,height:900}});
  page.on('console',m=>logs.push({name,type:m.type(),text:m.text()}));
  page.on('pageerror',e=>logs.push({name,type:'pageerror',text:e.stack||String(e)}));
  const url=new URL(base);url.searchParams.set('name',name);
  await page.goto(url.href);
  await page.getByLabel('Prepared resources').setInputFiles(bundle);
  await page.waitForFunction(()=>Number(document.body.dataset.horizonWorldDrawn)>0 && document.body.dataset.horizonPosition,{},{timeout:90000});
  return page;
 }
 const read=page=>page.evaluate(()=>({position:document.body.dataset.horizonPosition,peers:JSON.parse(document.body.dataset.horizonPeerPositions),vertices:Number(document.body.dataset.horizonPeerVertices),target:JSON.parse(document.body.dataset.horizonTarget),observed:JSON.parse(document.body.dataset.horizonObservedBlocks),error:document.querySelector('#error').textContent}));
 try {
  const prefix=process.env.HORIZON_WEB_NAME_PREFIX||'WebProof';
  const a=await join(prefix+'Alpha'); const b=await join(prefix+'Beta');
  await a.waitForFunction(()=>Number(document.body.dataset.horizonWorldPeers)>=1 && Number(document.body.dataset.horizonPeerVertices)>0,{},{timeout:30000});
  await b.waitForFunction(()=>Number(document.body.dataset.horizonWorldPeers)>=1 && Number(document.body.dataset.horizonPeerVertices)>0,{},{timeout:30000});
  await b.locator('#horizon-canvas').click();
  const before=(await read(b)).position;
  await b.keyboard.down('w');
  await b.waitForFunction(before=>{const old=before.split(',').map(Number);const now=document.body.dataset.horizonPosition.split(',').map(Number);return Math.hypot(now[0]-old[0],now[2]-old[2])>3;},before,{timeout:15000});
  await b.keyboard.up('w');
  const moved=(await read(b)).position.split(',').map(Number);
  await a.waitForFunction(at=>JSON.parse(document.body.dataset.horizonPeerPositions).some(([,p])=>Math.hypot(...p.map((n,i)=>n-at[i]))<0.25),moved,{timeout:15000});
  evidence.movement={before,after:(await read(b)).position,observer:await read(a)};
  await a.screenshot({path:output+'/peer.png'});
  await a.locator('#horizon-canvas').click();
  await a.keyboard.down('ArrowDown');
  await a.waitForFunction(()=>JSON.parse(document.body.dataset.horizonTarget)!==null,{},{timeout:15000});
  await a.keyboard.up('ArrowDown');
  // Look farther down so the intended block is near the player, inside both clients' columns.
  await a.keyboard.down('ArrowDown');await a.waitForTimeout(350);await a.keyboard.up('ArrowDown');
  const target=(await read(a)).target;
  if(!target)throw new Error('No block outline under the crosshair');
  const placed=target[0].map((n,i)=>n+target[1][i]);
  await a.getByLabel('Creative block').selectOption('minecraft:gold_block');
  await a.locator('#horizon-canvas').focus();
  await a.keyboard.press('f');
  const changed=page=>page.waitForFunction(pos=>JSON.parse(document.body.dataset.horizonObservedBlocks).some(([p,state])=>state!==0&&p.every((n,i)=>n===pos[i])),placed,{timeout:15000});
  await Promise.all([changed(a),changed(b)]);
  evidence.placement={pos:placed,a:await read(a),b:await read(b)};
  await a.waitForFunction(()=>document.body.dataset.horizonMeshPending==='0',{},{timeout:60000});
  await a.screenshot({path:output+'/placed.png'});
  // Reconnect the same world URL, importing the owned bytes again.
  await a.reload();await a.getByLabel('Prepared resources').setInputFiles(bundle);
  await a.waitForFunction(()=>Number(document.body.dataset.horizonWorldDrawn)>0,{},{timeout:90000});
  await a.locator('#horizon-canvas').click();
  // The server persists orientation along with the player. It may already be looking at the edit.
  await a.waitForFunction(pos=>{const target=JSON.parse(document.body.dataset.horizonTarget);return target && target[0].every((n,i)=>n===pos[i]);},placed,{timeout:15000});
  evidence.reconnect=await read(a);
  await a.keyboard.press('r');
  const broken=page=>page.waitForFunction(pos=>JSON.parse(document.body.dataset.horizonObservedBlocks).some(([p,state])=>state===0&&p.every((n,i)=>n===pos[i])),placed,{timeout:15000});
  await Promise.all([broken(a),broken(b)]);
  evidence.breaking={a:await read(a),b:await read(b)};
  await a.waitForFunction(()=>document.body.dataset.horizonMeshPending==='0',{},{timeout:60000});
  await a.screenshot({path:output+'/broken.png'});
  evidence.passed=!logs.some(m=>m.type==='pageerror'||/panicked|Invalid Shader|Error while parsing WGSL|validation error/i.test(m.text)) && !(await read(a)).error && !(await read(b)).error;
 } catch(error) {evidence.error=String(error);}
 finally {evidence.logs=logs;fs.writeFileSync(output+'/browser.json',JSON.stringify(evidence,null,2));await browser.close();}
 if(!evidence.passed)process.exitCode=1;
}
main().catch(error=>{console.error(error);process.exitCode=1;});
