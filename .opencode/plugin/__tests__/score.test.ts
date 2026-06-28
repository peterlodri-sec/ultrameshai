import { describe, test, expect } from 'bun:test';
import { isProtected, ebbinghausDecay, structuralBoost, scoreMessage } from '../kompress-ultra';

describe('isProtected', () => {
  test('last 5 messages are protected', () => {
    const messages = [{role:'assistant', content:'a'}, {role:'assistant', content:'b'},
      {role:'assistant', content:'c'}, {role:'assistant', content:'d'},
      {role:'assistant', content:'e'}];
    for (let i = 0; i < messages.length; i++) {
      expect(isProtected(messages[i], i, messages.length)).toBe(true);
    }
  });

  test('user messages are protected', () => {
    expect(isProtected({role:'user', content:'do this'}, 10, 20)).toBe(true);
  });

  test('code blocks are protected', () => {
    expect(isProtected({role:'assistant', content:'```rust\nfn main() {}```'}, 10, 20)).toBe(true);
  });

  test('error messages are protected', () => {
    expect(isProtected({role:'tool', content:'Error: something failed', type:'error'}, 10, 20)).toBe(true);
  });
});

describe('ebbinghausDecay', () => {
  test('recent messages score higher', () => {
    expect(ebbinghausDecay(0)).toBeCloseTo(1.0);
    expect(ebbinghausDecay(10)).toBeGreaterThan(ebbinghausDecay(20));
  });
});

describe('structuralBoost', () => {
  test('user role gets boost', () => {
    expect(structuralBoost({role:'user', content:'test'})).toBeGreaterThan(0.5);
  });

  test('code content gets boost', () => {
    expect(structuralBoost({role:'assistant', content:'```code```'})).toBeGreaterThan(0.5);
  });
});
