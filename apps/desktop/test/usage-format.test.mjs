import test from 'node:test';
import assert from 'node:assert/strict';
import {remainingPercent, windowLabel, resetCountdown} from '../src/usage-format.ts';

test('remaining usage is bounded and handles an exhausted or exceeded limit', () => {
  assert.equal(remainingPercent(23), 77);
  assert.equal(remainingPercent(100), 0);
  assert.equal(remainingPercent(105), 0);
});
test('labels use the reported window instead of assuming an hourly quota', () => {
  assert.equal(windowLabel(300, 'primary', 'en'), '5-hour window');
  assert.equal(windowLabel(10080, 'secondary', 'en'), 'Weekly limit');
  assert.equal(windowLabel(null, 'primary', 'en'), 'Short-term window');
  assert.equal(windowLabel(300, 'primary', 'fa'), 'بازهٔ ۵ ساعته');
});
test('countdowns preserve missing and passed reset states without inventing a refill', () => {
  assert.equal(resetCountdown(null, 100, 'en'), 'Reset time not reported');
  assert.equal(resetCountdown(100, 101, 'en'), 'Waiting for a fresh Codex update');
  assert.equal(resetCountdown(100 + 86400 + 7200, 100, 'en'), '1 d 2 h');
  assert.equal(resetCountdown(3700, 100, 'fa'), '۱ ساعت');
});
