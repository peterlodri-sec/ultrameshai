# Kompress-Ultra Integration Demo

This demonstrates how kompress-ultra integrates with opencode to provide intelligent context management.

## Setup
The kompress-ultra plugin is configured in `.opencode/opencode.json`:
```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": [
    "./.opencode/plugin/kompress-ultra.ts"
  ]
}
```

## Environment Variables
The plugin requires these environment variables in `.env`:
```
OVHCLOUD_API_KEY=...
OVHCLOUD_EMBEDDING_URL=...
OVHCLOUD_AI_API_KEY=...
OVHCLOUD_AI_BASE_URL=...
OVHCLOUD_AI_MODEL=...
MILVUS_URL=http://localhost:19530
MEMPALACE_DB=./mempalace.db
HONCHO_POLL_INTERVAL_MS=60000
```

## Key Features

### 1. Four Roles Implementation
- **Composer**: Enhances system prompts with brain patterns
- **Pruner**: Removes low-value messages while preserving safety floors
- **Rewriter**: Compresses content at different levels (Lite, Ultra, BrainBacked)
- **Circulator**: Stores pruned content for future retrieval

### 2. Smart Compression
```typescript
// Ultra compression removes filler words and structure
const input = "The user asked me to implement the authentication system with OAuth2. I would be happy to help with that."
const output = compressMessage(input, CompressionLevel.Ultra)
// Output: "user asked implement authentication system OAuth2"
```

### 3. Safety Floors
- Last 5 messages are always preserved
- User messages are always preserved
- Code blocks and error messages are preserved
- Critical system messages are preserved

### 4. Brain Integration
- Integrates with Milvus for pattern retrieval
- Monitors brain liveness status
- Stores pruned content for future retrieval
- Uses adaptive thresholds based on context density

## Testing
All integration tests pass:
```bash
bun test ./.opencode/plugin/__tests__/integration.test.ts
```

The plugin is now ready for use with opencode and will provide intelligent context management for all agents.