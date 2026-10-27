// Edge is used ONLY for viewport/visual parity: Moli's current CDP emulation
// accepts metrics overrides without updating the real layout viewport.
import assert from 'node:assert/strict';
import {readFile, writeFile} from 'node:fs/promises';

const pages=await(await fetch('http://127.0.0.1:19334/json/list')).json();
const page=pages.find(page=>page.type==='page');
const socket=new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
let id=0; const pending=new Map();
socket.addEventListener('message',event=>{
  const message=JSON.parse(event.data), request=pending.get(message.id);
  if(!request)return;pending.delete(message.id);clearTimeout(request.timer);
  if(message.error)request.reject(new Error(JSON.stringify(message.error)));else request.resolve(message.result);
});
const call=(method,params={})=>new Promise((resolve,reject)=>{
  const requestId=++id,timer=setTimeout(()=>reject(new Error(`CDP timeout: ${method}`)),15000);
  pending.set(requestId,{resolve,reject,timer});socket.send(JSON.stringify({id:requestId,method,params}));
});
async function evaluate(expression){
  const result=await call('Runtime.evaluate',{expression,returnByValue:true});
  if(result.exceptionDetails)throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
try {
  await call('Page.enable');
  await call('Page.addScriptToEvaluateOnNewDocument',{source:await readFile(new URL('./moli-fixture.js',import.meta.url),'utf8')});
  for(const width of [1120,760]) {
    await call('Emulation.setDeviceMetricsOverride',{width,height:1000,deviceScaleFactor:1,mobile:false});
    await call('Page.navigate',{url:'http://127.0.0.1:1421/'});
    let ready=false;
    for(let i=0;i<150;i++){if(await evaluate("document.querySelectorAll('.usage-window progress').length===2")){ready=true;break;}await new Promise(resolve=>setTimeout(resolve,100));}
    assert.equal(ready,true,'Usage panel did not load');
    assert.equal(await evaluate('innerWidth'),width);
    assert.equal(await evaluate('document.documentElement.scrollWidth<=innerWidth'),true,`Horizontal overflow at ${width}px`);
    assert.deepEqual(await evaluate('window.testErrors'),[]);
    const screenshot=await call('Page.captureScreenshot',{format:'png'});
    await writeFile(`target/edge-accounts-fa-${width}.png`,Buffer.from(screenshot.data,'base64'));
    console.log(`PASS: Edge visual viewport ${width}px; no overflow or JavaScript errors`);
  }
} finally { socket.close(); }
