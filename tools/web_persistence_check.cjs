const {chromium}=require(process.env.HORIZON_PLAYWRIGHT||'playwright');
const fs=require('node:fs');
async function main() {
 const output=process.env.HORIZON_WEB_PROOF||'.local/web-persistence-proof';
 const mode=process.env.HORIZON_WEB_PERSIST_MODE||'write';
 fs.mkdirSync(output,{recursive:true});
 const browser=await chromium.launch({channel:'chrome',headless:true,args:['--enable-unsafe-webgpu']});
 const logs=[];const evidence={passed:false,mode};
 try {
  const page=await browser.newPage({viewport:{width:1000,height:900}});
  page.on('console',m=>logs.push({type:m.type(),text:m.text()}));
  page.on('pageerror',e=>logs.push({type:'pageerror',text:e.stack||String(e)}));
  await page.goto(process.env.HORIZON_WEB_URL||'http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:8768&name=PersistProof');
  await page.getByLabel('Prepared resources').setInputFiles(process.env.HORIZON_WEB_BUNDLE||'.local/web-owned/play-resources.zip');
  await page.waitForFunction(()=>Number(document.body.dataset.horizonWorldDrawn)>0,{},{timeout:90000});
  await page.locator('#horizon-canvas').click();
  if(mode==='write') {
   await page.keyboard.down('ArrowDown');await page.waitForFunction(()=>JSON.parse(document.body.dataset.horizonTarget)!==null,{},{timeout:10000});await page.keyboard.up('ArrowDown');
   await page.keyboard.down('ArrowDown');await page.waitForTimeout(350);await page.keyboard.up('ArrowDown');
   const target=await page.evaluate(()=>JSON.parse(document.body.dataset.horizonTarget));
   const position=target[0].map((n,i)=>n+target[1][i]);
   await page.getByLabel('Creative block').selectOption('minecraft:diamond_block');await page.locator('#horizon-canvas').focus();await page.keyboard.press('f');
   await page.waitForFunction(pos=>JSON.parse(document.body.dataset.horizonObservedBlocks).some(([p,s])=>s!==0&&p.every((n,i)=>n===pos[i])),position,{timeout:15000});
   const state=await page.evaluate(pos=>JSON.parse(document.body.dataset.horizonObservedBlocks).find(([p,s])=>s!==0&&p.every((n,i)=>n===pos[i]))[1],position);
   evidence.block={position,state};
   fs.writeFileSync(output+'/block.json',JSON.stringify(evidence.block));
  } else if(mode==='read') {
   evidence.block=JSON.parse(fs.readFileSync(output+'/block.json','utf8'));
   await page.waitForFunction(({position,state})=>{const target=JSON.parse(document.body.dataset.horizonTarget);return target&&target[2]===state&&target[0].every((n,i)=>n===position[i]);},evidence.block,{timeout:15000});
  } else {throw new Error('Expected write or read mode');}
  evidence.target=await page.evaluate(()=>JSON.parse(document.body.dataset.horizonTarget));
  evidence.position=await page.locator('body').getAttribute('data-horizon-position');
  evidence.error=await page.locator('#error').textContent();
  await page.screenshot({path:output+'/'+mode+'.png'});
  evidence.passed=!evidence.error&&!logs.some(m=>m.type==='pageerror'||/panicked|Invalid Shader|validation error/i.test(m.text));
 } catch(error) {evidence.error=String(error);}
 finally {evidence.logs=logs;fs.writeFileSync(output+'/'+mode+'.json',JSON.stringify(evidence,null,2));await browser.close();}
 if(!evidence.passed)process.exitCode=1;
}
main().catch(error=>{console.error(error);process.exitCode=1;});
