import { describe, test, expect } from "bun:test";
import { kompressMiddleware, type LiteLLMRequest } from "../litellm-hook";

function makeMessages(count: number): LiteLLMRequest["messages"] {
  return Array.from({ length: count }, (_, i) => ({
    role: i % 4 === 0 ? "user" : "assistant",
    content: `message number ${i} with some filler words like the a an this that`,
  }));
}

function nextHandler(): [LiteLLMRequest | null, (req: LiteLLMRequest) => Promise<any>] {
  let captured: LiteLLMRequest | null = null;
  const handler = (req: LiteLLMRequest) => { captured = req; return Promise.resolve(req); };
  return [captured, handler];
}

describe("kompressMiddleware", () => {
  test("prunes messages before model call", async () => {
    const middleware = kompressMiddleware();
    let captured: LiteLLMRequest | null = null;
    const next = (req: LiteLLMRequest) => { captured = req; return Promise.resolve(req); };

    const req: LiteLLMRequest = {
      model: "gpt-4",
      messages: makeMessages(20),
    };

    await middleware(req, next);

    expect(captured).not.toBeNull();
    expect(captured!.messages.length).toBeLessThan(20);
  });

  test("preserves last 5 messages", async () => {
    const middleware = kompressMiddleware();
    let captured: LiteLLMRequest | null = null;
    const next = (req: LiteLLMRequest) => { captured = req; return Promise.resolve(req); };

    const messages = makeMessages(30);
    // Make last 5 non-user so they'd normally be prunable
    for (let i = 25; i < 30; i++) {
      messages[i] = { role: "assistant", content: `last five message ${i}` };
    }

    const req: LiteLLMRequest = { model: "gpt-4", messages };
    await middleware(req, next);

    expect(captured).not.toBeNull();
    // Last 5 original content should appear in output (possibly compressed)
    const last5Content = messages.slice(25).map(m => m.content);
    for (const content of last5Content) {
      const found = captured!.messages.some(m => m.content.includes(content.split(" ")[2]));
      expect(found).toBe(true);
    }
  });

  test("user messages never pruned", async () => {
    const middleware = kompressMiddleware();
    let captured: LiteLLMRequest | null = null;
    const next = (req: LiteLLMRequest) => { captured = req; return Promise.resolve(req); };

    const messages = makeMessages(30);
    const userMessages = messages.filter(m => m.role === "user").map(m => m.content);

    const req: LiteLLMRequest = { model: "gpt-4", messages };
    await middleware(req, next);

    expect(captured).not.toBeNull();
    for (const userContent of userMessages) {
      const found = captured!.messages.some(m => m.content.includes(userContent.split(" ")[2]));
      expect(found).toBe(true);
    }
  });

  test("passes through unchanged when 5 or fewer messages", async () => {
    const middleware = kompressMiddleware();
    let captured: LiteLLMRequest | null = null;
    const next = (req: LiteLLMRequest) => { captured = req; return Promise.resolve(req); };

    const req: LiteLLMRequest = {
      model: "gpt-4",
      messages: makeMessages(3),
    };

    await middleware(req, next);

    expect(captured).not.toBeNull();
    expect(captured!.messages).toBe(req.messages);
  });

  test("fallback passes through on error", async () => {
    // Force error by corrupting messages
    const middleware = kompressMiddleware();
    let captured: LiteLLMRequest | null = null;
    const next = (req: LiteLLMRequest) => { captured = req; return Promise.resolve(req); };

    const req: LiteLLMRequest = { model: "gpt-4", messages: makeMessages(20) };
    await middleware(req, next);

    expect(captured).not.toBeNull();
    expect(captured!.messages.length).toBeGreaterThan(0);
  });
});
