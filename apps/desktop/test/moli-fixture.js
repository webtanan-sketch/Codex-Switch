// Synthetic metadata only. Passed to Moli at document start; never bundled.
(() => {
  const scenario = new URL(location.href).searchParams.get('case');
  const profile = (id, name) => ({id,name,identity:{email:null,plan:'TEST'},createdAt:1700000000,lastUsedAt:null,lastSyncedAt:1700000000});
  const snapshot = {state:{profiles:[profile('a','Test A'),profile('b','Test B')],activeProfileId:'a',previousProfileId:null},
    settings:{vscodePath:null,codexPath:null},ipcReady:true,
    diagnostics:{windows:true,home:'TEST_HOME',authPresent:true,configPresent:true,storage:'unspecified',vscodePath:'TEST_Code.exe',codexPath:'TEST_codex.exe',recoveryPending:false}};
  window.testCalls = [];
  window.testErrors = [];
  addEventListener('error', event => window.testErrors.push(String(event.message)));
  addEventListener('unhandledrejection', event => window.testErrors.push(String(event.reason)));
  let nextCallback = 0;
  const callbacks = new Map();
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener:()=>{}};
  window.__TAURI_INTERNALS__ = {
    transformCallback(callback) { const id = ++nextCallback; callbacks.set(id,callback); return id; },
    unregisterCallback(id) { callbacks.delete(id); },
    async invoke(command, args = {}) {
      if (command === 'snapshot') return structuredClone(snapshot);
      if (command.startsWith('plugin:event|')) return 1;
      window.testCalls.push({command,args});
      if (command === 'read_usage') {
        const id = snapshot.state.activeProfileId;
        if (scenario === 'usage-error') throw new Error('TEST_UNAVAILABLE');
        if (scenario === 'usage-race' && id === 'a') await new Promise(resolve => setTimeout(resolve,3000));
        return {profileId:scenario === 'wrong-profile' ? 'unrelated' : id,checkedAt:Math.floor(Date.now()/1000),ordinaryUsageAllowed:true,
          windows:[{kind:'primary',usedPercent:id==='a'?23:38,durationMins:300,resetsAt:Math.floor(Date.now()/1000)+7200},
                   {kind:'secondary',usedPercent:61,durationMins:10080,resetsAt:Math.floor(Date.now()/1000)+172800}]};
      }
      if (command === 'configure_file_storage') { snapshot.diagnostics.storage = 'file'; return; }
      if (command === 'switch_profile') {
        if (snapshot.diagnostics.storage !== 'file') throw new Error('TEST_STORAGE_MODE');
        snapshot.state.previousProfileId=snapshot.state.activeProfileId;
        snapshot.state.activeProfileId=args.id; return;
      }
      if (command === 'open_vscode') return;
      throw new Error(`Unexpected fixture command ${command}`);
    }
  };
})();
