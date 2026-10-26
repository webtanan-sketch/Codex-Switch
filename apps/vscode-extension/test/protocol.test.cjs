const {test} = require('node:test');
const assert = require('node:assert/strict');
const {homeKey,parseEvent,LineDecoder} = require('../dist/protocol');
const sample = {version:1,type:'PROFILE_CHANGED',homeKey:homeKey('C:\\Users\\Test\\.codex'),profileId:'00000000-0000-4000-8000-000000000001',profileName:'PlayFood',timestamp:1};
test('accepts only allowlisted metadata',() => { assert.deepEqual(parseEvent(JSON.stringify(sample)),sample); assert.equal(parseEvent(JSON.stringify({...sample,auth:{}})),undefined); assert.equal(parseEvent(JSON.stringify({...sample,version:2})),undefined); });
test('handles fragmentation and multiple events',() => { const d = new LineDecoder(); const s = JSON.stringify(sample); assert.deepEqual(d.push(s.slice(0,7)),[]); assert.deepEqual(d.push(s.slice(7)+'\n'+s+'\n'),[sample,sample]); });
test('rejects oversized buffers and untrusted labels',() => { assert.throws(() => new LineDecoder().push('a'.repeat(32769))); assert.equal(parseEvent(JSON.stringify({...sample,profileName:'a\nb'})),undefined); assert.equal(parseEvent('not json'),undefined); });
test('normalizes Windows home paths identically to Rust',() => { assert.equal(homeKey('C:/Users/Test/.codex/'),homeKey('c:\\users\\test\\.codex')); assert.notEqual(homeKey('C:/Users/A'),homeKey('C:/Users/B')); });
