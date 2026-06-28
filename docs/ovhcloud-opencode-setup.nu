# ═══ OVHcloud AI Provider Setup ═══════════════════════════════════════════════
# For the no1 dev shell — add as OpenAI-compatible provider in OpenCode

## Step 1: Get OVH API credentials
# Go to: https://eu.api.ovh.com/createToken/
# Permissions: GET/POST/PUT/DELETE /ai/*, GET /me/*, GET/POST /cloud/project/*

## Step 2: Configure OVH CLI
mkdir -p ~/.ovh && cat > ~/.ovh/config.toml << 'EOF'
[default]
application_key = "YOUR_APP_KEY"
application_secret = "YOUR_APP_SECRET"
consumer_key = "YOUR_CONSUMER_KEY"
endpoint = "ovh-eu"
EOF

## Step 3: Get project ID
ovh api get /cloud/project

## Step 4: Deploy model (Qwen2.5-Coder-32B on H100)
PROJECT_ID="your-project-id"
ovh api post /ai/deploy/app \
  --name="qwen-coder-32b" \
  --projectId="$PROJECT_ID" \
  --image="ghcr.io/huggingface/text-generation-inference:2.3.1" \
  --gpu=1 \
  --env='[{"name":"MODEL_ID","value":"Qwen/Qwen2.5-Coder-32B-Instruct"},{"name":"NUM_SHARD","value":"1"},{"name":"MAX_INPUT_LENGTH","value":"4096"},{"name":"MAX_TOTAL_TOKENS","value":"8192"}]'

## Step 5: Get endpoint URL (wait for RUNNING status)
APP_ID=$(ovh api get /ai/deploy/app | jq -r '.[] | select(.name=="qwen-coder-32b") | .id')
ovh api get /ai/deploy/app/$APP_ID/endpoints
# → https://qwen-coder-32b-{hash}.{region}.ai.cloud.ovh.net

## Step 6: Wire into OpenCode
# Update the provider with real endpoint + key:
sqlite3 ~/.local/share/opencode/opencode.db "
UPDATE credential SET value = json_object(
  'baseUrl', 'https://YOUR-ENDPOINT.gra.ai.cloud.ovh.net/v1',
  'apiKey', 'stub-key-local-proxy'
) WHERE id = 'ovhcloud';
"

## Step 7: Test
curl -X POST https://YOUR-ENDPOINT.gra.ai.cloud.ovh.net/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer stub-key-local-proxy" \
  -d '{"model":"tgi","messages":[{"role":"user","content":"hi"}],"max_tokens":10}'

## Wiring into agent models
# In oh-my-opencode-slim.json, reference OVH models as:
#   "model": "ovhcloud/tgi"
#
# Note: TGI serves a single model per endpoint, so model name is just "tgi"
# or whatever the endpoint routes to. No per-model swapping needed.
