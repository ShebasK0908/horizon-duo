const {chromium}=require(process.env.HORIZON_PLAYWRIGHT || 'playwright');
const fs=require('node:fs');
const http=require('node:http');
const assert=require('node:assert/strict');
const output=process.env.HORIZON_WEB_PROOF || '.local/web-embed-proof';
const bundle=process.env.HORIZON_WEB_BUNDLE || '.local/web-owned/play-resources.zip';
const base=process.env.HORIZON_WEB_URL || 'http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:8767';

async function main() {
 fs.mkdirSync(output,{recursive:true});
 const browser=await chromium.launch({channel:'chrome',headless:true,args:['--enable-unsafe-webgpu']});
 const evidence={passed:false,proof:'Desktop Chrome cross-origin sandboxed iframe. This is not an X post or X client.',base,logs:[]};
 let server,page;
 try {
  page=await browser.newPage({viewport:{width:800,height:800}});
  page.on('console',m=>evidence.logs.push({type:m.type(),text:m.text()}));
  page.on('pageerror',e=>evidence.logs.push({type:'pageerror',text:String(e)}));
  const url=new URL(base);
  url.searchParams.set('name','Frame'+Date.now().toString(36));
  const sandbox='allow-scripts allow-same-origin allow-pointer-lock allow-forms';
  const parent='<!doctype html><title>Horizon iframe probe</title><iframe title="Horizon" sandbox="'+sandbox+'" src="'+url.href.replaceAll('&','&amp;').replaceAll('"','&quot;')+'" style="width:720px;height:720px;border:0"></iframe>';
  server=http.createServer((request,response)=>{
   response.writeHead(200,{'Content-Type':'text/html'});
   response.end(parent);
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const parentURL='http://127.0.0.1:'+server.address().port+'/';
  assert.notEqual(new URL(parentURL).origin,url.origin,'parent and game have different origins');
  evidence.parent=parentURL;
  evidence.sandbox=sandbox;
  await page.goto(parentURL);
  const frame=page.frameLocator('iframe');
  await frame.getByLabel('Prepared resources').waitFor({state:'visible',timeout:90000});
  await frame.getByLabel('Prepared resources').setInputFiles(bundle);
  const game=page.frames().find(f=>f.parentFrame()===page.mainFrame());
  assert(game,'game frame exists');
  await game.waitForFunction(()=>Number(document.body.dataset.horizonWorldDrawn)>0||document.querySelector('#error').textContent,{},{timeout:90000});
  assert(!await frame.locator('#error').textContent(),'game session starts without error');
  const state=()=>game.evaluate(()=>({...document.body.dataset}));
  evidence.initial=await state();
  await frame.locator('canvas').focus();
  await page.keyboard.down('w');
  await game.waitForFunction(before=>{
   const p=document.body.dataset.horizonPosition.split(',').map(Number);
   return Math.hypot(p[0]-before[0],p[2]-before[2])>1;
  },evidence.initial.horizonPosition.split(',').map(Number));
  await page.keyboard.up('w');
  await page.waitForTimeout(650);
  evidence.moved=await state();
  await page.keyboard.down('ArrowDown');
  await game.waitForFunction(()=>Number(document.body.dataset.horizonView.split(',')[1])>=40);
  await page.keyboard.up('ArrowDown');
  await game.waitForFunction(()=>JSON.parse(document.body.dataset.horizonTarget)!==null);
  await frame.getByLabel('Creative block').selectOption('minecraft:gold_block');
  const target=await game.evaluate(()=>JSON.parse(document.body.dataset.horizonTarget));
  evidence.placed=target[0].map((n,i)=>n+target[1][i]);
  await frame.locator('canvas').focus();
  await page.keyboard.press('f');
  await game.waitForFunction(pos=>JSON.parse(document.body.dataset.horizonObservedBlocks).some(([p,s])=>s===3830&&p.every((n,i)=>n===pos[i])),evidence.placed,{timeout:15000});
  await game.waitForFunction(()=>document.body.dataset.horizonMeshPending==='0',{},{timeout:60000});
  await page.screenshot({path:output+'/frame.png'});
  evidence.final=await state();
  evidence.error=await frame.locator('#error').textContent();
  assert(!evidence.error,evidence.error);
  assert(!evidence.logs.some(m=>m.type==='pageerror'||/panicked|Invalid Shader|validation error/i.test(m.text)),'no GPU or page errors');
  evidence.passed=true;
 } catch(error) {
  evidence.failure=error.stack||String(error);
  if(page) {
   evidence.frames=await Promise.all(page.frames().map(async frame=>({url:frame.url(),state:await frame.evaluate(()=>({data:{...document.body.dataset},error:document.querySelector('#error')?.textContent})).catch(String)})));
   await page.screenshot({path:output+'/failure.png'}).catch(()=>{});
  }
 } finally {
  await browser.close();
  if(server)await new Promise(resolve=>server.close(resolve));
  fs.writeFileSync(output+'/browser.json',JSON.stringify(evidence,null,2));
 }
 console.log(JSON.stringify(evidence,null,2));
 if(!evidence.passed)process.exitCode=1;
}
main().catch(error=>{console.error(error);process.exitCode=1;});
