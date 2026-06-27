# OVHcloud AI Deploy — Code Model Setup

## Quick Start

### 1. Install OVHcloud AI CLI

```bash
pip3 install ovh ovhai --break-system-packages
```

### 2. Create OVHcloud Application

Go to: https://eu.api.ovh.com/createToken/

**Required credentials:**
- `application_key`
- `application_secret`
- `consumer_key`

**Permissions needed:**
- `GET/POST/PUT/DELETE /ai/*`
- `GET /me/*`
- `GET/POST /cloud/project/*`

### 3. Configure OVHcloud CLI

```bash
# Create config directory
mkdir -p ~/.ovh

# Create config file
cat > ~/.ovh/config.toml << 'EOF'
[default]
application_key = "YOUR_APP_KEY"
application_secret = "YOUR_APP_SECRET"
consumer_key = "YOUR_CONSUMER_KEY"
endpoint = "ovh-eu"
EOF
```

### 4. Get Project ID

```bash
# List your Public Cloud projects
ovh api get /cloud/project

# Note the project ID (e.g., "1234567890abcdef")
```

### 5. Deploy Code Model

**Option A: Qwen2.5-Coder-32B (Recommended for 80GB)**

```bash
ovh api post /ai/deploy/app \
  --name="qwen-coder-32b" \
  --projectId="YOUR_PROJECT_ID" \
  --image="ghcr.io/huggingface/text-generation-inference:2.3.1" \
  --gpu=1 \
  --env='[{"name":"MODEL_ID","value":"Qwen/Qwen2.5-Coder-32B-Instruct"},{"name":"NUM_SHARD","value":"1"},{"name":"MAX_INPUT_LENGTH","value":"4096"},{"name":"MAX_TOTAL_TOKENS","value":"8192"}]'
```

**Option B: DeepSeek-Coder-V2-Lite (16B, faster)**

```bash
ovh api post /ai/deploy/app \
  --name="deepseek-coder-v2" \
  --projectId="YOUR_PROJECT_ID" \
  --image="ghcr.io/huggingface/text-generation-inference:2.3.1" \
  --gpu=1 \
  --env='[{"name":"MODEL_ID","value":"deepseek-ai/DeepSeek-Coder-V2-Lite-Instruct"},{"name":"NUM_SHARD","value":"1"}]'
```

**Option C: CodeLlama-70B (Max capacity, needs quantization)**

```bash
ovh api post /ai/deploy/app \
  --name="codellama-70b" \
  --projectId="YOUR_PROJECT_ID" \
  --image="ghcr.io/huggingface/text-generation-inference:2.3.1" \
  --gpu=1 \
  --env='[{"name":"MODEL_ID","value":"TheBloke/CodeLlama-70B-Instruct-AWQ"},{"name":"NUM_SHARD","value":"1"},{"name":"QUANTIZE","value":"awq"}]'
```

### 6. Monitor Deployment

```bash
# Check app status
ovh api get /ai/deploy/app/YOUR_APP_ID

# Wait for status: "RUNNING"

# Get endpoint URL
ovh api get /ai/deploy/app/YOUR_APP_ID/endpoints
```

### 7. Test the Model

```bash
# Get the endpoint URL
ENDPOINT=$(ovh api get /ai/deploy/app/YOUR_APP_ID/endpoints | jq -r '.[0].url')

# Test inference
curl -X POST "$ENDPOINT/generate" \
  -H "Content-Type: application/json" \
  -d '{
    "inputs": "def fibonacci(n):\n    ",
    "parameters": {
      "max_new_tokens": 256,
      "temperature": 0.7,
      "top_p": 0.95
    }
  }' | jq
```

## Auto-Scaling Setup

**Enable auto-scaling (1-3 replicas):**

```bash
ovh api put /ai/deploy/app/YOUR_APP_ID/scaling \
  --minReplicas=1 \
  --maxReplicas=3 \
  --targetCPUUtilization=70
```

**Cost savings with auto-scaling:**
- Night (8hrs): 1 replica = $24.56
- Day (16hrs): 2 replicas avg = $98.24
- **Total/day: $122.80** vs 24/7 fixed = $175.68 (**30% savings**)

## GPU Selection

| GPU | Model | VRAM | Price/hr | Best For |
|-----|-------|------|----------|----------|
| **H100** | h100-380 | 80GB | $2.99 | ⭐ Best perf/price |
| **A100** | a100-180 | 80GB | $3.07 | Great alternative |
| **L40S** | l40s-90 | 48GB | $1.80 | Budget 34B models |

## Model Recommendations

### For 80GB VRAM (H100/A100)

| Model | Size | Speed | Quality | Use Case |
|-------|------|-------|---------|----------|
| **Qwen2.5-Coder-32B** | 32B | Fast | ⭐⭐⭐⭐⭐ | Best overall |
| **DeepSeek-Coder-V2-Lite** | 16B | Very Fast | ⭐⭐⭐⭐ | Quick iterations |
| **CodeLlama-70B-AWQ** | 70B | Slow | ⭐⭐⭐⭐⭐ | Max quality (quantized) |
| **StarCoder2-15B** | 15B | Fast | ⭐⭐⭐⭐ | Good all-rounder |

### For 48GB VRAM (L40S)

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| **Qwen2.5-Coder-14B** | 14B | Fast | ⭐⭐⭐⭐ |
| **DeepSeek-Coder-6.7B** | 6.7B | Very Fast | ⭐⭐⭐ |
| **StarCoder2-7B** | 7B | Fast | ⭐⭐⭐ |

## Cost Estimates

**H100 80GB @ $2.99/hr:**
- 8 hrs/day × 30 days = **$718/month**
- 24/7 = **$2,153/month**
- With auto-scaling (1-3 replicas, avg 2) = **~$1,500/month**

**A100 80GB @ $3.07/hr:**
- 8 hrs/day × 30 days = **$737/month**
- 24/7 = **$2,232/month**

**L40S 48GB @ $1.80/hr:**
- 8 hrs/day × 30 days = **$432/month**
- 24/7 = **$1,314/month**

## Troubleshooting

**App stuck in "SCALING":**
```bash
# Check logs
ovh api get /ai/deploy/app/YOUR_APP_ID/logs

# Common issues:
# - Model too large for GPU (use quantized version)
# - Out of memory (reduce MAX_INPUT_LENGTH)
# - Image pull error (check image name)
```

**High latency:**
- Use `NUM_SHARD=1` for single GPU
- Reduce `MAX_INPUT_LENGTH` if not needed
- Consider smaller model (16B vs 32B)

**Model loading fails:**
- Check model exists on HuggingFace
- Use quantized version for 70B models (`-AWQ` or `-GPTQ`)
- Increase `MAX_INPUT_LENGTH` gradually

## Next Steps

1. **Get OVHcloud credentials** from https://eu.api.ovh.com/createToken/
2. **Choose GPU**: H100 (best) or A100 (alternative)
3. **Pick model**: Qwen2.5-Coder-32B recommended
4. **Deploy app** using commands above
5. **Test endpoint** with curl
6. **Enable auto-scaling** for cost savings

## Resources

- [OVHcloud AI Deploy Docs](https://docs.ovhcloud.com/en/guides/public-cloud/ai-machine-learning/ai-deploy/)
- [Text Generation Inference](https://github.com/huggingface/text-generation-inference)
- [Qwen2.5-Coder](https://huggingface.co/Qwen/Qwen2.5-Coder-32B-Instruct)
- [OVHcloud Pricing](https://www.ovhcloud.com/en/public-cloud/prices/)
