# The 8-Bug Deployment: A NixOS Odyssey

*Or: how we turned a 2-line config change into a 4-hour, 8-bug, cross-continent debugging session.*

---

## The Setup

One line in `hosts/dev-cx53/default.nix`:

```nix
webhookUrl = "https://proposal.vaked.dev/api/hooks/dogfeed";
```

One service: a self-improving data generation loop called `dogfeed`. Clone a repo, set two API keys, run a Bun script. What could go wrong?

Everything.

---

## Bug 1: `//home` is a symlink

**Symptom:** `error: path '//home' is a symlink`
**Root cause:** Nix 2.24+ rejects `path:` flake inputs through symlinks. macOS has `/home` as a symlink, and our Hetzner NixOS host also had it as one.
**Fix:** Every private flake input (`cloak-fetch`, `portail`) defaults to a no-op stub. The deploy script can override with the real path at build time.
**Lesson:** Never commit `path:/home/` or `path:/Users/` as a flake input. Use stubs + deploy-time overrides. If you must use a path input, `realpath` it first.

## Bug 2: Force-push reverts a merge

**Symptom:** `attribute 'crabcc-src' missing` + 10 cascading errors
**Root cause:** We force-pushed our branch to main, overwriting a previous PR that had archived `crabcc`. The archive commit (PR #49) was erased from history.
**Fix:** Re-archive crabcc from scratch. Remove all references from flake.nix, modules, and pkgs.
**Lesson:** Force-push is a chainsaw, not a scalpel. Before force-pushing, check what commits are on main that aren't on your branch. `git log origin/main --not HEAD` is your friend.

## Bug 3: The sudo password from hell

**Symptom:** `[sudo] password for dev@...` — but nobody knows the password
**Root cause:** The dev user was created by a previous AI session. The password hash was sops-encrypted in `secrets/users.yaml` and never set imperatively.
**Fix:** Hetzner rescue mode, mount the disk, add NOPASSWD sudoers rule. Root password: `HLdJF7TVPwR4` (sops-encrypted for next time).
**Lesson:** For headless NixOS boxes, always set `security.sudo.extraRules = [ { users = ["dev"]; commands = [{ command = "ALL"; options = ["NOPASSWD"]; }]; } ];` from day one. `wheelNeedsPassword = false` is not enough for non-interactive SSH sudo.

## Bug 4: Wrong entrypoint

**Symptom:** `dogfeed.service: Module not found "src/index.ts"`
**Root cause:** `src/index.ts` re-exports modules but has no `main()` function. The real entrypoint is `src/cli.ts`.
**Fix:** Change `exec bun src/index.ts` to `exec bun src/cli.ts`.
**Lesson:** When renaming entrypoints, update ALL references — including NixOS systemd service definitions 10,000km away.

## Bug 5: Read-only filesystem

**Symptom:** `fatal: could not create work tree dir': Read-only file system`
**Root cause:** `ProtectSystem = "strict"` in the systemd service hardening makes all of `/var` read-only. The `ExecStartPre` git clone failed.
**Fix:** Add `ReadWritePaths = "/var/lib/dogfeed"` to the service config.
**Lesson:** NixOS systemd hardening is aggressive. If your service needs to write to its own directory, you MUST whitelist it with `ReadWritePaths`. The alternative (`ProtectSystem = "full"`) is still read-only for non-whitelisted paths.

## Bug 6: Sops key mismatch

**Symptom:** `failed to decrypt: 0 successful groups required, got 0`
**Root cause:** The sops secrets were encrypted with the operator's age key (`age1zcwylk...`). The NixOS host doesn't have this key. sops-nix imported the SSH host key, but it's a different key.
**Fix:** Generate a dedicated age key for the host, add its public key to `.sops.yaml`, re-encrypt all 13 secrets, and set the key file via tmpfiles (copy from nix store → `/var/lib/sops-nix/key.txt`).
**Lesson:** Never assume sops-encrypted secrets will decrypt on a remote host. Always include the host's key as an additional recipient. Use `sops --rotate -i --add-age <host-key>` for each secret file.

## Bug 7: Docker Hub timeout

**Symptom:** `docker-honcho-redis.service: status 255/EXCEPTION`
**Root cause:** The tailnet-only host pulls Docker images from `registry-1.docker.io` which is slow/unreachable over the Hetzner → tailnet route. The default `TimeoutStartSec = 0` (infinity) should be enough, but the pull was cancelled by Docker's client timeout.
**Fix:** Pre-pull all images in a separate oneshot systemd service with `TimeoutSec = 300`. Wire it as `before` and `wantedBy` the docker container services.
**Lesson:** Docker Hub is not reliable on tailnet-only hosts. Pre-pull images explicitly. Consider using a local registry mirror.

## Bug 8: The nth merge conflict

**Symptom:** `syntax error, unexpected '||', expecting 'inherit'` at `||||||| 883b4ed`
**Root cause:** After force-pushing main, merging 4 stale PRs left conflict markers in the file. The `|||||||` merge-base marker was missed.
**Fix:** `grep -n "||||||\|<<<<<<\|======\|>>>>>>" flake.nix` — find and remove them.
**Lesson:** After force-pushing, check for leftover conflict markers. A simple grep saves 40 minutes of debugging.

---

## The final state

After 8 bugs, 1800+ lines of changes, 20+ commits, and 4 PRs merged:

| Service | Status | Notes |
|---------|--------|-------|
| dogfeed | ✅ | Self-improving data loop |
| honcho | ✅ | Memory service (db + reddis + api + deriver) |
| aticd | ✅ | Binary cache |
| tailscale | ✅ | Mesh |
| second-brain | ✅ | Cloudflare sync |
| sops | ✅ | All 14 secrets decrypt |
| portail-runner | ❌ | Needs reboot (userns fix deployed) |

## The meta-lesson

NixOS is not harder than imperative config. It's different. The bugs weren't caused by NixOS's declarative model — they were caused by:

1. **No CI** — every bug hit production first
2. **No staging** — one command deploys to the only server
3. **Siloed secrets** — sops key not shared with the host
4. **Force-push** — rewriting history creates cascading issues

The fix isn't "learn Nix better." The fix is **CI that builds + deploys**, **automated smoke tests**, **secrets distribution**, and **never force-pushing main**.

*Built in public at [github.com/peterlodri-sec/nix-base](https://github.com/peterlodri-sec/nix-base). Dogfeed dataset at [huggingface.co/datasets/PeetPedro/ultrawhale-dogfood](https://huggingface.co/datasets/PeetPedro/ultrawhale-dogfood).*
