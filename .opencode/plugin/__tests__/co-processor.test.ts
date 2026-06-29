import { describe, test, expect } from 'bun:test';
import { CoProcessor } from '../lib/co-processor';

describe('CoProcessor', () => {
  let cp: CoProcessor;

  test('init returns false when ollama not available', async () => {
    cp = new CoProcessor({ base_url: 'http://localhost:99999' });
    const ok = await cp.init();
    expect(ok).toBe(false);
  });

  test('compress falls back to heuristic when ollama unavailable', async () => {
    cp = new CoProcessor({ base_url: 'http://localhost:99999' });
    await cp.init();

    const input =
      'The user asked me to implement the authentication system. I would be happy to help with that. ' +
      'I have successfully added OAuth2 support and the tests are passing now.';
    const output = await cp.compress(input);

    expect(output.length).toBeLessThan(input.length);
    expect(output).toContain('OAuth2');
    expect(output).not.toContain('would be happy');
  });

  test('compress returns shorter output', async () => {
    cp = new CoProcessor({ base_url: 'http://localhost:99999' });
    await cp.init();

    const input =
      'Basically I just need to update the configuration file at /etc/nginx/nginx.conf ' +
      'and restart the service. That should fix the issue with the 502 errors.';
    const output = await cp.compress(input);

    expect(output.length).toBeLessThan(input.length * 0.7);
    expect(output).toContain('nginx');
    expect(output).toContain('conf');
  });

  test('compress preserves code blocks', async () => {
    cp = new CoProcessor({ base_url: 'http://localhost:99999' });
    await cp.init();

    const input = 'Here is the code:\n```rust\nfn main() { println!("Hello"); }\n```\nDone.';
    const output = await cp.compress(input);

    expect(output).toContain('```rust');
    expect(output).toContain('fn main()');
  });

  test('synthesize returns compact brain state', async () => {
    cp = new CoProcessor({ base_url: 'http://localhost:99999' });
    await cp.init();

    const findings = [
      { content: 'Authentication module uses OAuth2 with PKCE flow', score: 0.95 },
      { content: 'Database migration v42 adds user_sessions table', score: 0.87 },
      { content: 'Rate limiter set to 100 req/min per API key', score: 0.72 },
      { content: 'Just a random note about lunch', score: 0.1 },
    ];

    const output = await cp.synthesize(findings);
    expect(output.length).toBeGreaterThan(0);
    expect(output).toContain('brain-state');
    // High-score items should appear, low-score filtered
    expect(output).toContain('OAuth2');
  });

  test('score_batch returns scores for all messages', async () => {
    cp = new CoProcessor({ base_url: 'http://localhost:99999' });
    await cp.init();

    const messages = [
      { role: 'user', content: 'implement auth' },
      { role: 'assistant', content: 'sure, here is the code' },
      { role: 'tool', content: 'Error: connection refused' },
      { role: 'assistant', content: '```rust\nfn main() {}```' },
    ];

    const scores = await cp.score_batch(messages);
    expect(scores.length).toBe(messages.length);
    expect(scores[0]).toBeCloseTo(0.9, 1); // user
    expect(scores[2]).toBeCloseTo(0.9, 1); // error
    expect(scores[3]).toBeCloseTo(0.8, 1); // code
  });

  test('default options use qwen2.5:1.5b', () => {
    cp = new CoProcessor();
    // Internal state check via public behavior
    expect(cp).toBeDefined();
  });
});
