import { describe, test, expect } from 'bun:test';
import { compressMessage, CompressionLevel } from '../../lib/rewriter';

describe('compressMessage', () => {
  test('verbatim preserves content exactly', () => {
    const input = 'The user asked me to implement the authentication system with OAuth2.';
    expect(compressMessage(input, CompressionLevel.Verbatim)).toBe(input);
  });

  test('lite drops articles and filler', () => {
    const input = 'The user asked me to implement the authentication system with OAuth2. I would be happy to help with that.';
    const output = compressMessage(input, CompressionLevel.Lite);
    expect(output).not.toContain('The ');
    expect(output).not.toContain('would be happy');
    expect(output).toContain('authentication');
    expect(output).toContain('OAuth2');
  });

  test('ultra produces fragments', () => {
    const input = 'The authentication system implementation is now complete. I have successfully added OAuth2 support and the tests are passing.';
    const output = compressMessage(input, CompressionLevel.Ultra);
    expect(output.length).toBeLessThan(input.length * 0.5);
    expect(output).toContain('OAuth2');
  });

  test('code blocks are never compressed', () => {
    const input = 'Here is the code:\n```rust\nfn main() { println!("Hello"); }\n```\nDone.';
    const output = compressMessage(input, CompressionLevel.Ultra);
    expect(output).toContain('```rust');
    expect(output).toContain('fn main()');
  });

  test('markdown fences stay balanced', () => {
    const input = 'Before:\n```rust\ncode1\n```\nAfter:\n```rust\ncode2\n```';
    const output = compressMessage(input, CompressionLevel.Ultra);
    const fenceCount = (output.match(/```/g) || []).length;
    expect(fenceCount % 2).toBe(0);
  });
});
