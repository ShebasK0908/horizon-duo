const {chromium} = require(process.env.HORIZON_PLAYWRIGHT || 'playwright');
const fs = require('node:fs');
const path = require('node:path');

async function main() {
    const output = process.env.HORIZON_WEB_PROOF || '.local/web-proof';
    fs.mkdirSync(output, {recursive: true});
    const browser = await chromium.launch({channel: 'chrome', headless: true,
        args: ['--enable-unsafe-webgpu']});
    const logs = [];
    let passed = false;
    try {
        const page = await browser.newPage({viewport: {width: 1000, height: 780}});
        await page.addInitScript(() => {Error.stackTraceLimit = 50;});
        page.on('console', message => {
            logs.push({type: message.type(), text: message.text()});
            console.log(message.type(), message.text());
        });
        page.on('pageerror', error => logs.push({type: 'pageerror', text: error.stack}));
        const bundle = process.env.HORIZON_WEB_BUNDLE;
        const url = process.env.HORIZON_WEB_URL || ('http://127.0.0.1:8766' + (bundle ? '/?owned' : '/'));
        await page.goto(url, {waitUntil: 'commit'});
        if (bundle) {
            await page.getByLabel('Prepared resources').waitFor({state: 'visible'});
            await page.getByLabel('Prepared resources').setInputFiles(bundle);
        }
        const expectedError = process.env.HORIZON_WEB_EXPECT_ERROR;
        if (expectedError) {
            await page.getByText(expectedError, {exact: true}).waitFor({timeout: 10000});
        } else {
            await page.locator('body[data-horizon-frames="3"]').waitFor({timeout: 60000});
        }
        await page.screenshot({path: path.join(output, 'fixture.png'), timeout: 10000});
        const result = await page.evaluate(() => ({
            frames: document.body.getAttribute('data-horizon-frames'),
            quads: document.body.getAttribute('data-horizon-quads'),
            webgpu: !!navigator.gpu,
            error: document.querySelector('#error').textContent,
        }));
        passed = expectedError ? result.error === expectedError && result.frames === null : result.frames === '3' && (bundle ? Number(result.quads) > 0 : result.quads === '18') && !result.error
            && !logs.some(entry => entry.type === 'pageerror' || /Invalid Shader|Error while parsing WGSL|panicked|validation error/i.test(entry.text));
        fs.writeFileSync(path.join(output, 'browser.json'), JSON.stringify({passed, result, logs}, null, 2));
    } catch (error) {
        fs.writeFileSync(path.join(output, 'browser.json'), JSON.stringify({passed, error: String(error), logs}, null, 2));
    } finally {
        await browser.close();
    }
    if (!passed) process.exitCode = 1;
}
main().catch(error => {console.error(error); process.exitCode = 1;});
