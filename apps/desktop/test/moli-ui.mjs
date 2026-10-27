// Run against a local preview and an isolated `moli serve --layout` instance
// started with --document-start-script-file test/moli-fixture.js.
import assert from 'node:assert/strict';
import {mkdir, readFile, writeFile} from 'node:fs/promises';

const base = process.env.MANAGER_TEST_URL || 'http://127.0.0.1:1421';
const endpoint = process.env.MOLI_TEST_URL || 'http://127.0.0.1:19333';
const pages = await (await fetch(`${endpoint}/json/list`)).json();
const socket = new WebSocket(pages[0].webSocketDebuggerUrl);
await new Promise((resolve,reject) => { socket.addEventListener('open',resolve,{once:true}); socket.addEventListener('error',reject,{once:true}); });
let nextId=0;
const pending=new Map();
socket.addEventListener('message',event=>{
  const message=JSON.parse(event.data);
  if (!message.id) return;
  const request=pending.get(message.id); if (!request) return;
  pending.delete(message.id); clearTimeout(request.timer);
  if (message.error) request.reject(new Error(JSON.stringify(message.error))); else request.resolve(message.result);
});
function call(method,params={}) {
  return new Promise((resolve,reject)=>{
    const id=++nextId;
    const timer=setTimeout(()=>{pending.delete(id);reject(new Error(`CDP timeout: ${method}`));},15000);
    pending.set(id,{resolve,reject,timer}); socket.send(JSON.stringify({id,method,params}));
  });
}
async function evaluate(expression) {
  const result=await call('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function waitFor(expression) {
  const deadline=Date.now()+15000;
  do { if (await evaluate(expression)) return; await new Promise(resolve=>setTimeout(resolve,100)); } while(Date.now()<deadline);
  throw new Error(`UI condition timed out: ${expression}; ${await evaluate('document.body.innerText')}`);
}
async function click(label,scope='document') {
  // Moli refreshes its on-demand layout on screenshot capture. New React nodes
  // retain zero geometry until this paint has happened.
  await call('Page.captureScreenshot',{format:'png'});
  const point=await evaluate(`(()=>{const root=${scope};const button=[...root.querySelectorAll('button')].find(b=>b.textContent.trim()===${JSON.stringify(label)});if(!button)throw new Error('Button not found');const r=button.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2,disabled:button.disabled};})()`);
  assert.equal(point.disabled,false,`Button disabled: ${label}`);
  await call('Input.dispatchMouseEvent',{type:'mousePressed',button:'left',clickCount:1,x:point.x,y:point.y});
  await call('Input.dispatchMouseEvent',{type:'mouseReleased',button:'left',clickCount:1,x:point.x,y:point.y});
}
async function navigate(scenario='') {
  await call('Page.navigate',{url:`${base}/${scenario ? `?case=${scenario}` : ''}`});
  await waitFor("document.querySelectorAll('.card').length===2");
}
async function noErrors() { assert.deepEqual(await evaluate('window.testErrors'),[]); }

try {
  await call('Page.addScriptToEvaluateOnNewDocument',{source:await readFile(new URL('./moli-fixture.js',import.meta.url),'utf8')});
  await call('Page.enable'); await call('Runtime.enable');
  await call('Emulation.setDeviceMetricsOverride',{width:1120,height:1400,deviceScaleFactor:1,mobile:false});
  await navigate();
  await waitFor("document.querySelectorAll('.usage-window progress').length===2");
  assert.equal(await evaluate("document.querySelector('.usage-window progress').getAttribute('value')"),'77');
  assert.equal(await evaluate('document.documentElement.dir'),'rtl');
  assert.match(await evaluate("document.querySelector('.usage-panel').textContent"),/هفتگی/);
  await click('تعویض حساب',"document.querySelectorAll('.card')[1]");
  await waitFor("Boolean(document.querySelector('.storage-prompt'))");
  assert.equal(await evaluate("window.testCalls.some(c=>c.command==='configure_file_storage'||c.command==='switch_profile')"),false);
  await click('انصراف');
  await waitFor("!document.querySelector('.storage-prompt')");
  assert.equal(await evaluate("window.testCalls.some(c=>c.command==='switch_profile')"),false);
  await click('تعویض حساب',"document.querySelectorAll('.card')[1]");
  await waitFor("Boolean(document.querySelector('.storage-prompt'))");
  await click('فعال‌سازی و تعویض حساب');
  await waitFor("document.querySelector('.card.selected h3')?.textContent==='Test B' && document.querySelector('.usage-window progress')?.getAttribute('value')==='62'");
  assert.deepEqual(await evaluate("window.testCalls.filter(c=>c.command==='configure_file_storage'||c.command==='switch_profile').map(c=>c.command)"),['configure_file_storage','switch_profile']);
  await click('باز کردن VS Code ↗',"document.querySelectorAll('.card')[1]");
  await waitFor("window.testCalls.some(c=>c.command==='open_vscode'&&c.args.id==='b')");
  await noErrors();
  console.log('PASS: quota rendering, consent/cancel, file-storage setup, switching, and VS Code action');
  await mkdir('target',{recursive:true});
  const screenshot=await call('Page.captureScreenshot',{format:'png'});
  await writeFile('target/moli-accounts-fa.png',Buffer.from(screenshot.data,'base64'));
  console.log('PASS: Moli layout screenshot saved; viewport-size parity is checked separately');
  await call('Emulation.setDeviceMetricsOverride',{width:1120,height:1400,deviceScaleFactor:1,mobile:false});
  await navigate('usage-error');
  await waitFor("Boolean(document.querySelector('.usage-error'))");
  assert.equal(await evaluate("document.querySelectorAll('.usage-window progress').length"),0);
  await noErrors();
  console.log('PASS: unavailable quota never appears as zero consumption');
  await navigate('wrong-profile');
  await waitFor("Boolean(document.querySelector('.usage-error'))");
  assert.equal(await evaluate("document.querySelectorAll('.usage-window progress').length"),0);
  await noErrors();
  console.log('PASS: another account quota is rejected');
  await navigate('usage-race');
  await click('تعویض حساب',"document.querySelectorAll('.card')[1]");
  await waitFor("Boolean(document.querySelector('.storage-prompt'))");
  await click('فعال‌سازی و تعویض حساب');
  await waitFor("document.querySelector('.usage-window progress')?.getAttribute('value')==='62'");
  await new Promise(resolve=>setTimeout(resolve,3500));
  assert.equal(await evaluate("document.querySelector('.usage-window progress').getAttribute('value')"),'62');
  await noErrors();
  console.log('PASS: late quota response cannot replace the newly selected account quota');
} finally { socket.close(); }
