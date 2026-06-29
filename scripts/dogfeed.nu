# dogfeed.nu — nushell harness for the dogfeed data generation loop
#
# Usage:
#   nu scripts/dogfeed.nu run              — start the loop
#   nu scripts/dogfeed.nu stats            — show loop statistics
#   nu scripts/dogfeed.nu doctor           — check config + connectivity
#   nu scripts/dogfeed.nu test-llm         — test LLM connection
#   nu scripts/dogfeed.nu export           — export to JSONL
#   nu scripts/dogfeed.nu push             — manual push to HuggingFace
#   nu scripts/dogfeed.nu help             — show this help

def main [] {
  help
}

def "main run" [
  --topics: string = ""         # Comma-separated topics (empty = defaults)
  --interval: int = 30          # Seconds between iterations
  --ralph                     # Enable Ralph reflection
  --compress                  # Enable kompress-ultra compression
  --db: string = "./dogfeed.db" # SQLite database path
  --hf-repo: string = ""        # HuggingFace dataset repo
  --push-every: int = 50        # Push after N records
] {
  let env_vars = [
    { name: "OPENROUTER_KEY", value: ($env.OPENROUTER_KEY? | default "") }
    { name: "HF_TOKEN", value: ($env.HF_TOKEN? | default "") }
  ]

  print "🐕 Starting dogfeed loop..."
  print $"  Interval: ($interval)s"
  print $"  DB: ($db)"
  print $"  Compress: ($compress)"
  print $"  Ralph: ($ralph)"
  print ""

  cd (each { |t| $t.value } | where { |t| $t.name == "PWD" } | first | get value | path join "packages/dogfeed")

  with-env $env_vars {
    bun $"src/index.ts"
  }
}

def "main stats" [
  --db: string = "./dogfeed.db"
] {
  if not ($db | path exists) {
    print $"Database not found: ($db)"
    return
  }
  print "📊 Loop Statistics"
  print ""
  # Query SQLite directly
  let total = (sqlite3 $db "SELECT COUNT(*) FROM records" | into int)
  let pushed = (sqlite3 $db "SELECT COUNT(*) FROM records WHERE pushed = 1" | into int)
  let tokens = (sqlite3 $db "SELECT COALESCE(SUM(tokens_in + tokens_out), 0) FROM records" | into int)
  let errors = (sqlite3 $db "SELECT COUNT(*) FROM events WHERE level = 'ERROR'" | into int)
  let topics = (sqlite3 $db "SELECT DISTINCT topic FROM records ORDER BY topic" | split row "\n" | where { |t| $t != "" })

  print $"  Records: ($total) generated, ($pushed) pushed"
  print $"  Tokens:  ($tokens)"
  print $"  Errors:  ($errors)"
  print $"  Topics:  ($topics | length) — ($topics | str join ', ')"
}

def "main doctor" [] {
  print "🏥 dogfeed doctor"
  print ""

  # Check bun
  let bun_version = (try { bun --version | str trim } catch { "NOT FOUND" })
  print $"  bun: ($bun_version)"

  # Check OpenRouter key
  let or_key = ($env.OPENROUTER_KEY? | default "")
  if ($or_key | str length) > 0 {
    print $"  OPENROUTER_KEY: set ($or_key | str substring 0..8)..."
  } else {
    print "  OPENROUTER_KEY: NOT SET (LLM calls will fail)"
  }

  # Check HF token
  let hf_token = ($env.HF_TOKEN? | default "")
  if ($hf_token | str length) > 0 {
    print $"  HF_TOKEN: set ($hf_token | str substring 0..8)..."
  } else {
    print "  HF_TOKEN: NOT SET (publishing will fail)"
  }

  # Check HF repo
  let hf_repo = ($env.HF_REPO? | default "")
  if ($hf_repo | str length) > 0 {
    print $"  HF_REPO: ($hf_repo)"
  } else {
    print "  HF_REPO: NOT SET"
  }

  print ""
  print "✅ Doctor check complete"
}

def "main test-llm" [] {
  let key = ($env.OPENROUTER_KEY? | default "")
  if ($key | str length) == 0 {
    print "❌ OPENROUTER_KEY not set"
    return
  }
  print "🧪 Testing OpenRouter connection..."
  let response = (http post
    --headers [{name: "Authorization", value: $"Bearer ($key)"}, {name: "Content-Type", value: "application/json"}]
    "https://openrouter.ai/api/v1/chat/completions"
    { model: "qwen/qwen-2.5-7b-instruct:free", messages: [{ role: "user", content: "Say hello in one word" }], max_tokens: 10 }
    | get choices.0.message.content)
  print $"  Response: ($response)"
  print "✅ LLM connection OK"
}

def "main export" [
  --db: string = "./dogfeed.db"
  --output: string = "./dogfeed-export.jsonl"
] {
  if not ($db | path exists) {
    print $"Database not found: ($db)"
    return
  }
  print "📦 Exporting to JSONL..."
  let records = (sqlite3 --json $db "SELECT * FROM records WHERE pushed = 0 ORDER BY id")
  $records | each { |r|
    {
      id: $"dogfeed-($r.created_at | str replace --all '[^0-9]' '' | str substring 0..14)-($r.id)"
      topic: $r.topic
      question: $r.question
      answer: ($r.compressed_answer? | default $r.answer)
      model: $r.model
      tokens_in: ($r.tokens_in | into int)
      tokens_out: ($r.tokens_out | into int)
      source: "dogfeed-loop"
      topic_category: ($r.topic | str replace --all ' ' '-')
      created_at: $r.created_at
    }
  } | each { |r| $r | to json } | str join "\n" | save --force $output
  print $"  Exported ($records | length) records to ($output)"
}

def "main push" [
  --repo: string = ""
  --token: string = ""
] {
  let repo = (if ($repo | str length) > 0 { $repo } else { $env.HF_REPO? | default "" })
  let token = (if ($token | str length) > 0 { $token } else { $env.HF_TOKEN? | default "" })

  if ($repo | str length) == 0 {
    print "❌ HF_REPO not set (use --repo or set HF_REPO env)"
    return
  }
  if ($token | str length) == 0 {
    print "❌ HF_TOKEN not set (use --token or set HF_TOKEN env)"
    return
  }

  print $"📤 Pushing to ($repo)..."
  # Export first, then upload via hf cli
  main export --output /tmp/dogfeed-push.jsonl
  let file_size = ((open /tmp/dogfeed-push.jsonl | lines | length) | into int)
  print $"  ($file_size) records to push"
  print "  Use: hf upload $repo /tmp/dogfeed-push.jsonl data/loop-export.jsonl --repo-type dataset"
}

def "main help" [] {
  print "dogfeed — self-improving data generation loop"
  print ""
  print "Commands:"
  print "  nu scripts/dogfeed.nu run              — start the loop"
  print "  nu scripts/dogfeed.nu run --ralph      — enable Ralph reflection"
  print "  nu scripts/dogfeed.nu stats            — show loop statistics"
  print "  nu scripts/dogfeed.nu doctor           — check config + connectivity"
  print "  nu scripts/dogfeed.nu test-llm         — test LLM connection"
  print "  nu scripts/dogfeed.nu export           — export unpushed to JSONL"
  print "  nu scripts/dogfeed.nu push             — manual push to HuggingFace"
  print "  nu scripts/dogfeed.nu help             — show this help"
  print ""
  print "Environment:"
  print "  OPENROUTER_KEY  — OpenRouter API key"
  print "  HF_TOKEN        — HuggingFace token"
  print "  HF_REPO         — HuggingFace dataset repo"
}
