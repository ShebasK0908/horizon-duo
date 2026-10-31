const {chromium}=require(process.env.HORIZON_PLAYWRIGHT || 'playwright');
const fs=require('node:fs');
const output=process.env.HORIZON_WEB_PROOF || '.local/web-tick-proof';
const base=process.env.HORIZON_WEB_URL || 'http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:8767';
const bundle=process.env.HORIZON_WEB_BUNDLE || '.local/web-owned/play-resources.zip';
const tickMs=50,frameDelayMs=250;
const boundaryTicks=Math.ceil(frameDelayMs/tickMs)+2;
async function main() {
 fs.mkdirSync(output,{recursive:true});
 const browser=await chromium.launch({channel:'chrome',headless:true,args:['--enable-unsafe-webgpu']});
 const evidence={passed:false,base,windows:[],errors:[]};
 try {
  const page=await browser.newPage();
  let ticks=0;
  page.on('websocket',socket=>socket.on('framesent',frame=>{
   if(Buffer.isBuffer(frame.payload) && frame.payload.equals(Buffer.from([2,0,13])))ticks++;
  }));
  page.on('pageerror',error=>evidence.errors.push(String(error)));
  await page.addInitScript(()=>{
   const requestFrame=window.requestAnimationFrame.bind(window);
   window.horizonTestFrameDelay=0;
   window.requestAnimationFrame=callback=>requestFrame(time=>{
    if(window.horizonTestFrameDelay)setTimeout(()=>callback(performance.now()),window.horizonTestFrameDelay);
    else callback(time);
   });
  });
  const url=new URL(base);
  url.searchParams.set('name','Tick'+Date.now().toString(36));
  await page.goto(url.href);
  await page.getByLabel('Prepared resources').waitFor({state:'visible'});
  await page.getByLabel('Prepared resources').setInputFiles(bundle);
  await page.waitForFunction(()=>Number(document.body.dataset.horizonWorldDrawn)>0,{},{timeout:90000});
  for(const delay of [0,frameDelayMs]) {
   await page.evaluate(delay=>window.horizonTestFrameDelay=delay,delay);
   await page.waitForTimeout(600);
   const before=ticks,start=Date.now();
   await page.waitForTimeout(2500);
   const elapsed=Date.now()-start;
   const sample={renderDelayMs:delay,elapsedMs:elapsed,ticks:ticks-before,expectedTicks:elapsed/tickMs};
   evidence.windows.push(sample);
   if(Math.abs(sample.ticks-sample.expectedTicks)>boundaryTicks)throw new Error('Simulation tick cadence follows rendering delay');
  }
  evidence.errorText=await page.locator('#error').textContent();
  evidence.passed=!evidence.errorText && evidence.errors.length===0;
 } catch(error) {evidence.error=String(error);}
 finally {fs.writeFileSync(output+'/browser.json',JSON.stringify(evidence,null,2));await browser.close();}
 console.log(JSON.stringify(evidence,null,2));
 if(!evidence.passed)process.exitCode=1;
}
main().catch(error=>{console.error(error);process.exitCode=1;});
