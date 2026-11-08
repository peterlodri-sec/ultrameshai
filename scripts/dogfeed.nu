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

# --- helpers -----------------------------------------------------------------

# Resolve the dogfeed package directory from this script's location so the
# harness works regardless of $env.PWD. The script lives at
# $REPO_ROOT/scripts/dogfeed.nu and the package at $REPO_ROOT/packages/dogfeed.
def dogfeed-pkg-dir [] {
    let script_path = ($env.FILE_PARSED?.path | default "")
    if $script_path == "" {
        # Fallback: $PWD's parent is repo root, packages/dogfeed is the target
        $env.PWD | path join ".." | path join "packages" | path join "dogfeed" | path expand
    } else {
        $script_path
        | path dirname   # scripts/
        | path dirname   # repo root
        | path join "packages/dogfeed"
        | path expand
    }
}

# Resolve HF token from explicit arg, $env.HF_TOKEN, or ~/.config/huggingface/token
def resolve-hf-token [explicit: string = ""] {
    if ($explicit | str length) > 0 {
        return $explicit
    }
    let env_tok = ($env.HF_TOKEN? | default "")
    if ($env_tok | str length) > 0 {
        return $env_tok
    }
    let cfg_path = ($env.HOME | path join ".config/huggingface/token")
    if ($cfg_path | path exists) {
        return (open --raw $cfg_path | str trim)
    }
    let cache_path = ($env.HOME | path join ".cache/huggingface/token")
    if ($cache_path | path exists) {
        return (open --raw $cache_path | str trim)
    }
    ""
}

# A small record with the env vars the loop reads. Nushell's with-env
# wants a record ({name=value}), not a list of {name,value} rows.
def loop-env [
    --db: string = "./dogfeed.db"
    --hf-repo: string = ""
] {
    {
        OPENROUTER_KEY: ($env.OPENROUTER_KEY? | default "")
        HF_TOKEN:       (resolve-hf-token)
        DOGFEED_DB:     $db
        DOGFEED_HF_REPO: $hf_repo
    }
}

# --- top-level help ---------------------------------------------------------

def main [] {
    help
}

def "main run" [
    --topics: string = ""         # Comma-separated topics (empty = defaults)
    --interval: int = 30          # Seconds between iterations
    --ralph                       # Enable Ralph reflection
    --compress                    # Enable kompress-ultra compression
    --db: string = "./dogfeed.db" # SQLite database path
    --hf-repo: string = ""        # HuggingFace dataset repo
    --push-every (-p): int = 50   # Push after N records
] {
    let pkg = (dogfeed-pkg-dir)
    if not ($pkg | path exists) {
        print -e $"(ansi red_bold)Error: dogfeed package directory not found at ($pkg).(ansi reset)"
        print "Set DOGFEED_PKG_DIR or run from the ultrameshai repo root."
        exit 1
    }

    print "🐕 Starting dogfeed loop..."
    print $"  Package: ($pkg)"
    print $"  Interval: ($interval)s"
    print $"  DB: ($db)"
    print $"  Compress: ($compress)"
    print $"  Ralph: ($ralph)"
    print ""

    let push_every_str = ($push_every | into string)
    let interval_str = ($interval | into string)
    let env_record = (
        loop-env --db $db --hf-repo $hf_repo
        | upsert DOGFEED_INTERVAL $interval_str
        | upsert DOGFEED_PUSH_EVERY $push_every_str
    )
    let env_record = (if $ralph    { $env_record | upsert DOGFEED_RALPH_EVERY "1" } else { $env_record })
    let env_record = (if $compress { $env_record | upsert DOGFEED_COMPRESS "1" } else { $env_record })
    let env_record = (if ($topics | str length) > 0 { $env_record | upsert DOGFEED_TOPICS $topics } else { $env_record })

    cd $pkg
    with-env $env_record { ^bun run }
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
    let hf_token = (resolve-hf-token)
    if ($hf_token | str length) > 0 {
        let pfx = ($hf_token | str substring 0..8)
        print $"  HF_TOKEN: set ($pfx)..."
    } else {
        print "  HF_TOKEN: NOT SET (publishing will fail)"
    }

    # Check HF repo
    let hf_repo = ($env.HF_REPO? | default ($env.DOGFEED_HF_REPO? | default ""))
    if ($hf_repo | str length) > 0 {
        print $"  HF_REPO: ($hf_repo)"
    } else {
        print "  HF_REPO: NOT SET"
    }

    # Check local Ollama
    let ollama_ok = (try { http get --max-time 2sec "http://localhost:11434/api/tags" | get models | length } catch { -1 })
    if $ollama_ok >= 0 {
        print $"  Ollama:   reachable — ($ollama_ok) local models"
    } else {
        print "  Ollama:   not reachable on :11434"
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

# Helper: parse a sqlite3 --json result into a table.
def parse-rows [json: string] {
    $json | from json
}

# Build a JSONL row for export. The shape mirrors publish.ts:recordsToJSONL()
# so anything exported here can be uploaded as a drop-in replacement for the
# auto-published batches. Both `answer` (raw) and `compressed_answer` (Lite
# pass) are included, and `role` reflects which is present — never the
# other way around. See review P2 on PR #3.
def to-jsonl-row [r] {
    let role = (if ($r.compressed_answer? | default "" | str length) > 0 { "pruner" } else { "generator" })
    {
        id: $"dogfeed-($r.created_at | str replace --all '[^0-9]' '' | str substring 0..14)-($r.id)"
        topic: $r.topic
        question: $r.question
        answer: $r.answer
        compressed_answer: ($r.compressed_answer? | default null)
        model: $r.model
        tokens_in: ($r.tokens_in | into int)
        tokens_out: ($r.tokens_out | into int)
        role: $role
        source: "dogfeed-loop"
        topic_category: ($r.topic | str lowercase | str replace --all ' ' '-')
        created_at: $r.created_at
    } | to json -r
}

def "main export" [
    --db: string = "./dogfeed.db"
    --output: string = "./dogfeed-export.jsonl"
    --all                          # include already-pushed rows too
] {
    if not ($db | path exists) {
        print $"Database not found: ($db)"
        return
    }
    print "📦 Exporting to JSONL..."
    let where = (if $all { "" } else { "WHERE pushed = 0" })
    let raw = (sqlite3 --json $db $"SELECT * FROM records ($where) ORDER BY id")
    if (($raw | str trim | str length) == 0) {
        "" | save --force $output
        print $"  Exported 0 records to ($output)"
        return
    }
    let records = (parse-rows $raw)
    let rows = ($records | each { |r| (to-jsonl-row $r) })
    $rows | str join "\n" | save --force $output
    print $"  Exported ($records | length) records to ($output)"
}

def "main push" [
    --repo: string = ""
    --token: string = ""
    --batch: int = 50
    --db: string = "./dogfeed.db"
] {
    let repo = (if ($repo | str length) > 0 { $repo } else { $env.HF_REPO? | default ($env.DOGFEED_HF_REPO? | default "") })
    let token = (resolve-hf-token $token)

    if ($repo | str length) == 0 {
        print "❌ HF_REPO not set (use --repo or set HF_REPO env)"
        return
    }
    if ($token | str length) == 0 {
        print "❌ HF_TOKEN not set (use --token, set HF_TOKEN env, or ~/.config/huggingface/token)"
        return
    }

    let db_abs = ($db | path expand)
    if not ($db_abs | path exists) {
        print $"❌ Database not found: ($db_abs)"
        return
    }

    let pkg = (dogfeed-pkg-dir)
    print $"📤 Pushing up to ($batch) unpushed records from ($db_abs) to ($repo)..."

    cd $pkg
    with-env {
        DOGFEED_DB: $db_abs
        DOGFEED_HF_REPO: $repo
        HF_TOKEN: $token
        DOGFEED_BATCH: ($batch | into string)
    } {
        ^bun -e '
          import { DogfeedDB, pushAll } from "./src/index.ts";
          const db = new DogfeedDB(process.env.DOGFEED_DB!);
          const pushed = await pushAll(
            db,
            process.env.DOGFEED_HF_REPO!,
            process.env.HF_TOKEN!,
            Number(process.env.DOGFEED_BATCH || "50"),
            { webhookUrl: process.env.DOGFEED_WEBHOOK_URL }
          );
          db.close();
          if (pushed === 0) {
            console.log("  No unpushed records pushed.");
          } else {
            console.log(`  ✓ Pushed ${pushed} records (mirrored data/latest.jsonl + data/stats.json)`);
          }
        '
    }
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
