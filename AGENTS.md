# AGENTS.md — stack

> **Canonical ecosystem rules live in `~/ecosystem/INVARIANTS.md` and `~/ecosystem/AGENTS-HARNESS.md`** (Mass / Momentum / Moxie internal masters). This repository is Apache-2.0 public, so the relevant canonical blocks are **inlined verbatim below** to keep external clones self-contained. Changes land in the masters first, then propagate here via the deliberate sync procedure (see `~/ecosystem/survey.sh`).
>
> **Mirrors this repo's `CLAUDE.md`** on substance. Before editing code in this repo, read `./CLAUDE.md` — it carries the repo-local layout, commands, doctrine, and conventions. `AGENTS.md` and `CLAUDE.md` must not diverge in facts; they may differ in structure and voice.
>
> **Model target.** gpt-5-codex family (current preferred: gpt-5.3-codex or latest; fallback: gpt-5.2-codex), `reasoning_effort=high` or `xhigh` for non-trivial work (Pro-class). Terse, declarative voice (Russian mathematical school; see §IV of the inlined invariants below). No LLM attribution on commits (§VI).

---

Mass Protocol EZ Stack — the open-source (Apache-2.0) zone operator kit.
10 crates. What operators fork to deploy their own sovereign jurisdictional
runtime. The runtime is distributed as a Docker image referenced from
`deploy/`; the proprietary tree is not a build dependency of this repository.

This file is the Codex / OpenAI-optimized agent contract. Its factual content
mirrors `CLAUDE.md` in this repository; its format is engineered for Codex 5.x
token economy and failure-mode mitigation. Both files are authoritative; if they
diverge, reconcile by reading the code and updating both.

---

<!-- BEGIN NO-DESTRUCTIVE-GIT (canonical rule — do not remove or edit) -->

## NON-NEGOTIABLE: No destructive git — ever

Applies across every Mass / Momentum / Moxie repo
(moxie, moxie-whitepaper, moxie/web, kernel, kernel worktrees, centcom, stack, lex,
gstore, momentum, momentum-dev, momentum-research, momentum-docs, mass-webapp,
mass-bom, api-gateway, attestation-engine, templating-engine, starters,
organization-info, investment-info, treasury-info, identity-info, consent-info,
governance-info, institutional-world-model-whitepaper,
programmable-institutions-whitepaper, and every other Mass/Momentum/Moxie repo).

**Forbidden commands (non-exhaustive):**

- `git commit` from a subagent (main thread commits only — subagents stage only)
- `git push` in any form, any branch (main thread pushes only)
- `git reset --hard`, `git reset --keep`, or any `git reset` that moves HEAD
- `git checkout` of a shared checkout, `git switch`, `git restore`
- `git stash` in any form (including `pop`, `drop`, `apply`, `clear`)
- `git clean` in any form (`-f`, `-fd`, `-x`, …)
- `git rebase` in any form (including interactive)
- `git branch -D`, `git branch --delete --force`
- `git worktree remove --force`
- `git update-ref`, `git filter-branch`, `git filter-repo`
- `rm -rf` on anything git-tracked

**Required:**

- Agents stage changes only (`git add <path>`). The main thread alone commits and pushes.
- Parallel work uses `git worktree add <unique-path> -b <unique-branch> origin/<base>` and operates inside that isolated path. Never mutate the shared checkout's HEAD.
- Merge conflicts are resolved via merge commits — never via `reset`, `stash`, or `checkout`.
- If a destructive op seems necessary, STOP and escalate to the user. Do not proceed.

**Additive alternatives (always safe):** `git worktree add`, `git revert <commit>`,
`git diff > patch.diff`, `git merge` (no-ff or default), `git fetch`.

This rule survives context compression. Every agent spawned in this repo inherits it.

**Incident reference:** 2026-04-16, Agent 5 (conservation invariants) ran
`git reset --hard --no-recurse-submodules` inside its isolated worktree despite a
"DO NOT commit. Stage only." instruction. The prompt failed to enumerate the
forbidden-command list verbatim. Lesson: the list above must be pasted into every
agent prompt — no paraphrasing, no abbreviation.

<!-- END NO-DESTRUCTIVE-GIT -->

<!-- BEGIN MULTI-AGENT-CONCURRENCY (canonical rule — do not remove or edit) -->

## NON-NEGOTIABLE: Multi-agent concurrency via worktrees

Many local agents run against this repo simultaneously from a single main thread.
They MUST share the repo without destructive interaction. The only safe model:

**Every non-trivial agent operates in its own git worktree:**

```
git worktree add <unique-path> -b <unique-branch> origin/<base-branch>
cd <unique-path>
# ... do work, stage changes ...
# main thread reviews, merges (merge commit only), pushes
```

- `<unique-path>` must be unique per agent (e.g. `/tmp/agent-<id>` or a path that embeds a UUID/task-id). Never reuse paths across agents.
- `<unique-branch>` must be unique per agent (e.g. `agent/<task-id>` or `frontier/<name>-<short-sha>`). Never reuse branch names.
- `<base-branch>` is whatever the user has checked out on main thread (typically `develop` or `main`).

**Rules for concurrent agents:**

1. An agent operates ONLY inside its own worktree path. Never `cd` out of it into the shared checkout. Never read/write files in the shared checkout (that path belongs to the main thread and possibly other agents).
2. An agent never touches HEAD of the shared checkout. No `git checkout`, `git switch`, `git reset`, `git rebase` anywhere.
3. An agent never mutates another agent's worktree or branch.
4. An agent stages changes inside its worktree (`git add`). It does NOT commit. The main thread commits after reviewing the staged changes (agents cannot reliably write good commit messages under a shared history, and commits from parallel agents race on the branch ref).
5. An agent never pushes. Only the main thread pushes.
6. When an agent finishes, its worktree and branch stay until the main thread merges or the user explicitly authorizes cleanup. Do NOT `git worktree remove` your own worktree on exit — the harness cleans up when appropriate.
7. If an agent hits a conflict with another agent's work, it reports the conflict to the main thread and stops. It does NOT resolve the conflict via reset/checkout/stash.
8. If an agent needs to read another repo (cross-repo context), it reads files directly (Read tool) — it does NOT `git checkout` or `git worktree add` in a repo it is not assigned to.

**Read-only agents** (audit, explore, documentation search) may operate in the shared checkout without worktree isolation, because they do not write. They still never run any git command that mutates state.

**File-locking guidance for agents sharing the main checkout (read-only only):**

- Use Read, Grep, Glob freely.
- Do NOT use Edit, Write, or Bash commands that write files in the shared checkout.
- If you find something that needs a write, report it — don't write.

**If any of the above becomes infeasible, STOP and escalate to the user.**
Never silently break the concurrency invariant.

<!-- END MULTI-AGENT-CONCURRENCY -->

---

## License invariant (LOAD-BEARING)

Every file in this repository is Apache-2.0. Every contribution must remain
Apache-2.0. **If a change would introduce proprietary content — code, spec fragments,
partner-specific configuration, non-Apache licensed dependencies — STOP and
escalate to the user.** The open-source boundary is the product.

- **READS allowed:** sibling Apache-2.0 repos (`~/lex`, `~/op`, `~/gstore`).
- **WRITES allowed:** only Apache-2.0 zone-operator artifacts inside this repo.
- **NEVER:** import any proprietary source tree; reproduce closed-source
  crates by name; call deployed microservices directly — go through the
  runtime's HTTP surface; add non-Apache-2.0 dependencies.

The proprietary runtime is distributed as a Docker image referenced from
`deploy/docker-compose.yaml`. It is not a build dependency. Foundational
types (`ComplianceDomain`, `CanonicalBytes`, `sha256_digest`) are shared
through the public `mez-canonical` crate at `~/lex/crates/mez-canonical`
so the wire format is identical across the open/closed boundary without
any code copy.

---

## Ecosystem

This repo is one lane in Mass / Momentum / Moxie. Lane: Apache-2.0 zone operator kit.

The four open-source whitelist repositories (Apache-2.0):

- `~/stack` — zone-operator deployment kit (this repo)
- `~/lex` — Lex: typed jurisdictional rules (4 crates, 742 tests)
- `~/op` — Op: typed compliance-carrying workflows
- `~/gstore` — Merkle-authenticated temporal graph store

Foundational types shared across the four are in `~/lex/crates/mez-canonical`
(`CanonicalBytes`, `sha256_digest`, `ComplianceDomain`).

Closed-source companion trees exist on the operator's local machine; their
identities, paths, and crate names must NEVER appear in artifacts shipped
from this repository. CI enforces this via
`.github/workflows/forbidden-strings.yml`.

---

## Build & verify

```bash
# Compile check
cargo check --workspace

# Tests
cargo test --workspace

# Lint (zero warnings)
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check
```

Run all three (check / test / clippy) after any Rust change.

---

## Architecture

`stack` is the open-source deployment kit for the four-repo public set
(lex + op + gstore + this). Its role is to give third-party zone operators
a working runtime they can fork and deploy without any proprietary build
dependency.

- **10 crates, Apache-2.0**, workspace-managed
- **Type vocabulary** shared with the public `mez-canonical` crate
  (`~/lex/crates/mez-canonical`) for compliance domains, canonical
  serialization, and content digests, so corridors and passports remain
  wire-compatible across the four-repo set
- **Zero proprietary build dependencies.** If a dependency appears in
  `Cargo.lock` that is not Apache-2.0 / MIT / BSD-licensed, it is a license
  violation
- **Consumers:** zone operators (governments, private zones, pilot
  jurisdictions) who want a deployable runtime without proprietary licensing

---

## Hard rules

- **No LLM credit in git commits.** NEVER include `Co-Authored-By` lines
  referencing Claude, Opus, GPT, Codex, or any LLM in commit messages. The
  author is the human operator.
- **No destructive git** — see sentinel block above.
- **License invariant** — Apache-2.0 everywhere, no exceptions.
- **No proprietary imports** — never path-depend on closed-source trees,
  never copy code from any proprietary source.
- **No direct microservice calls** — go through the runtime's HTTP surface.
- **Deployment model** — `develop` is dev staging, `main` is prod staging.
  Never push `main` / `master`. Agents push nothing; only the main thread
  pushes.

---

## Key files / structure

```
stack/
├── CLAUDE.md       # Mirrored to this AGENTS.md
├── AGENTS.md       # This file (Codex-optimized)
├── Cargo.toml      # Workspace root (10 members)
└── crates/         # 10 Apache-2.0 crates
```

See `CLAUDE.md` for the canonical authorial voice. If counts or paths drift,
update both files together.

---

## Common tasks

| Task | Protocol |
|------|----------|
| New crate | (1) Confirm purpose belongs in Apache-2.0 zone-operator kit, not proprietary kernel. (2) Add to `Cargo.toml` workspace. (3) Apache-2.0 license header on every new file. (4) `cargo check --workspace` + `cargo test --workspace` + `cargo clippy --workspace -- -D warnings`. |
| New dependency | (1) Verify the crate's license is Apache-2.0 / MIT / BSD (never GPL/AGPL or proprietary). (2) Add to the relevant `Cargo.toml`. (3) Record license in any SBOM / NOTICE file if present. |
| Mirror a type from `mez-core` | (1) Only mirror structurally — copy the shape, not the source text. (2) Reference `mez-core` via wire format (e.g., JSON schema, Borsh layout), not path dependency. (3) Add a test that round-trips across the wire to catch drift. |
| Escalation | If a change cannot be done without importing proprietary code or non-Apache deps — STOP and escalate. |

---

## Codex cognitive calibration

- This repo is **small (10 crates)**. Prefer reading the whole workspace
  `Cargo.toml` before asserting about crate layout.
- Do not assume any file exists because it exists in `~/kernel`. The two
  codebases are deliberately disjoint. `cat` before referencing.
- When in doubt about whether a feature belongs here or in `~/kernel`, the
  default answer is `~/kernel`. This repo only gets the minimum needed for
  a self-hosting zone deployment.
- Never generate Co-Authored-By lines for LLMs in commit messages.
