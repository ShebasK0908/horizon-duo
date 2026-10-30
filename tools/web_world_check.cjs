const {chromium} = require(process.env.HORIZON_PLAYWRIGHT || 'playwright');
const fs = require('node:fs');
async function main() {
    const output = process.env.HORIZON_WEB_PROOF || '.local/web-world-proof';
    fs.mkdirSync(output, {recursive:true});
    const browser = await chromium.launch({channel:'chrome',headless:true,args:['--enable-unsafe-webgpu']});
    const logs=[];
    let passed=false;
    try {
        const page=await browser.newPage({viewport:{width:1000,height:780}});
        await page.addInitScript(()=>{Error.stackTraceLimit=30;});
        page.on('console',m=>{logs.push({type:m.type(),text:m.text()});console.log(m.type(),m.text());});
        page.on('pageerror',e=>logs.push({type:'pageerror',text:e.stack}));
        await page.goto(process.env.HORIZON_WEB_URL || 'http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:8767&name=BrowserOne');
        await page.getByLabel('Prepared resources').waitFor({state:'visible'});
        await page.getByLabel('Prepared resources').setInputFiles(process.env.HORIZON_WEB_BUNDLE || '.local/web-owned/world-resources.zip');
        await page.waitForFunction(()=>Number(document.body.dataset.horizonWorldChunks)>0 && Number(document.body.dataset.horizonWorldDrawn)>0,{},{timeout:60000});
        await page.screenshot({path:output+'/world.png'});
        const result=await page.evaluate(()=>({chunks:document.body.dataset.horizonWorldChunks,sections:document.body.dataset.horizonWorldSections,
            resident:document.body.dataset.horizonWorldResident,drawn:document.body.dataset.horizonWorldDrawn,error:document.querySelector('#error').textContent}));
        passed=Number(result.chunks)>0 && Number(result.drawn)>0 && !result.error && !logs.some(m=>m.type==='pageerror' || /panicked|Invalid Shader|Error while parsing WGSL|validation error/i.test(m.text));
        fs.writeFileSync(output+'/browser.json',JSON.stringify({passed,result,logs},null,2));
    } catch(error) {fs.writeFileSync(output+'/browser.json',JSON.stringify({passed,error:String(error),logs},null,2));}
    finally {await browser.close();}
    if(!passed)process.exitCode=1;
}
main().catch(error=>{console.error(error);process.exitCode=1;});
