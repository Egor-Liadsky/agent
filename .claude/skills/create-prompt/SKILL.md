---
name: create-prompt
description: Turns a vague task description into a ready, detailed prompt with four sections — Role, Goal, Steps, Report — asking only about gaps that would change the prompt fundamentally. Use for «составь промпт», «напиши промпт», «сделай промпт для», «промпт с ролью и отчётом», «составь роль», or /create-prompt.
---

# Create a prompt

## Job

Turn a vague request into a precise spec for another LLM. Think "what must
the executor know so it doesn't guess", not "how to phrase it nicely".

Output: one self-contained prompt with four sections — Role, Goal, Steps,
Report. Test: two different models reading it in a clean session do the
same work and report the same way. The role is a way to set bounds (who,
end state, order, what to return), not a character description.

## Sections of the output prompt

Exactly four, in this order. No fifth section, no empty or padded ones.

### Role

Who the executor is and what they own. Stack and tools: Rust edition, repo
and crate, key libs (axum, sqlx, ratatui, tokio). Hard bounds: what not to
do without explicit user consent.

Bounds must be checkable by reading a diff. Not "keep architecture clean"
but "`crates/core` doesn't depend on `clap`, `ratatui`, `crossterm` or other
terminal crates". Not "handle errors carefully" but "tell error causes apart
via `downcast_ref::<AgentError>()`, not message text". Not "don't touch
extra stuff" but "new provider = new `Agent` trait impl, not a caller edit".

No filler like "world-class expert", "think step by step", "this is very
important" — no effect, only length.

### Goal

End state and how to tell the work is done. Describe the result, not the
process: not "work on the doc" but "all sections written and match the code".

If there is a source of truth (doc, spec, issue), name it by file or link,
plus the rule for when it disagrees with reality.

### Steps

Numbered, fixed order, at least five:

1. **Read** — what to read before the first edit (`CLAUDE.md`, repo README,
   `openspec/changes/<name>/`, existing code and tests).
2. **Scope** — which repo (`agent-cli` or `agent-sever`), crates, modules,
   files are touched; what must not be created.
3. **Implement** — order across crates/modules/sections and why. If both
   core and service change, follow `CLAUDE.md`: `agent-cli/crates/core`
   first, push to `main`, then service `Cargo.lock`.
4. **Check** — exact full commands and the dir to run them from (not "run
   tests"; `cargo` fails in the umbrella root). On failure: quote the
   shortest decisive output line and don't claim success.
5. **Record** — where to write decisions before reporting: OpenSpec
   `design.md` or `tasks.md`, README, commit body.

### Report

1. What was done — by meaning, not a file list.
2. Checks as run: command and result; skipped steps named as skipped.
3. Decisions and deliberately deferred items.
4. One concrete next step, not a wish list.

## Algorithm

1. Map the request onto slots: executor, stack, bounds; end state and source
   of truth; step order and check commands; report form and where to record
   decisions.
2. Mark empty slots as blocking (answer changes the prompt's core) or
   non-blocking (a sane default works).
3. Before asking, read the repo: root `CLAUDE.md`, README and `Cargo.toml`
   of the target submodule, active `openspec/changes/`. Don't ask what files
   or the request already answer. If files contradict the request, say so and
   use the fact from files.
4. Close blocking slots in one `AskUserQuestion` call: max 3–4 questions,
   each with options and one marked recommended.
5. Fill other gaps with defaults and list them as "Assumptions" under the
   prompt so the user can object.
6. Output the whole prompt in one code block.

## Don't

- Do the task itself — only write the prompt.
- Leave `<...>` placeholders for the user to fill.
- Invent project facts: ask or mark as an assumption.
- Interrogate: if the request is complete, zero questions.

## Language

Write the prompt in the language the user speaks in the chat. Code,
identifiers, file names, CLI commands, flags, env vars and exact error texts
stay as is.

## Example

Request: "write a prompt for whoever will fix the failing service tests".

Repo reading gave: `agent-sever` = `agentd`, axum, edition 2024, SQLite via
`sqlx`, migrations in `migrations/*.sql`; contract tests in `src/tests.rs`,
provider mocked by `wiremock`; run `cargo test` from `agent-sever`; core
patched locally via `[patch]` to `../agent-cli/crates/core`. Blocking slots:
what the executor may change, where to record decisions. One
`AskUserQuestion` call:

1. "What may the executor change?" — tests and service code if the bug is
   there (recommended); plus core in `agent-cli`; tests only, list code bugs.
2. "Where to record decisions?" — commit body (recommended); new OpenSpec
   change with `design.md`; nowhere, chat report only.

User picked the recommended options. Result (shown in English here; the real
output follows the user's language):

```
## Role

Test engineer for agentd in /Users/egor_lyadskiy/ai/agent-sever: Rust
edition 2024, axum, tokio, SQLite via sqlx, /v1 contract tests in
src/tests.rs, provider mocked by wiremock. agentcore is a git dep, locally
patched via [patch] in .cargo/config.toml to ../agent-cli/crates/core.
You own making cargo test pass fully.

Bounds (break only with explicit user consent):

- a failing test is not deleted, marked #[ignore] or weakened into an
  always-passing check;
- no edits in ../agent-cli: a core bug is described in the report and the
  test stays failing;
- no new deps in Cargo.toml, no version changes;
- applied migrations in migrations/ are not edited — schema changes only
  via a new migration file;
- the error envelope { "error": { "code", "message", "request_id" } } and
  endpoint HTTP codes don't change to make a test pass;
- no real provider in tests, no real AGENTD_UPSTREAM_API_KEY: wiremock only;
- a service code bug is fixed in code, not in the test expectation.

## Goal

cargo test in agent-sever passes: zero failures, no new #[ignore]. Each fix
is explainable — clear whether test or code was wrong. Source of truth:
service README.md and openspec/specs/; if README, spec and code disagree,
ask the user, don't pick silently.

## Steps

1. Read. /Users/egor_lyadskiy/ai/CLAUDE.md and README.md endpoint sections,
   then from agent-sever run cargo test --no-fail-fast and list all
   failures: test name, line, kind (compile error, panic, assert_eq!
   mismatch). Edit nothing until the list is ready.
2. Scope. For each failure name the src/ module and decide: bug in test,
   service code or core. Don't touch other modules, don't create new ones.
3. Implement. Compile errors first (no test runs until the crate builds),
   then panics and value mismatches. One test at a time; after each fix run
   the full cargo test to catch regressions.
4. Check. Final run from agent-sever: cargo build, then cargo test. On
   failure quote the shortest decisive line (error[E…] with path and line,
   or test name with first panic line) and don't claim success.
5. Record. Decisions go in the commit body in English: which tests were
   fixed, test or code bug, which alternative was rejected and why. List
   unfixed core bugs separately.

## Report

1. Done: how many tests were fixed and what bugs were found, by meaning.
2. Checks: results of cargo build and cargo test; skipped ones named.
3. Decisions and deferred items — e.g. an agentcore bug to fix in
   agent-cli, or a test that passes but doesn't check what its name says.
4. One concrete next step.
```
