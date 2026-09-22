---
name: fix-changes
description: Splits working-tree changes in agent-cli, agent-sever and the umbrella repo into meaningful commits, runs cargo build and cargo test, bumps the service Cargo.lock to the pushed core commit after core changes, pushes submodules to their origin and commits the new submodule pointers in the umbrella repo. Use for «закоммить изменения», «разложи по коммитам», «зафиксируй и запушь», «обнови подмодули», «сделай коммиты и PR», or /fix-changes.
---

# Commit and push changes

## Context

Umbrella repo `/Users/egor_lyadskiy/ai` with two Rust (edition 2024) submodules:

- `agent-cli` — workspace: `agentcore` (`crates/core`), `agentupstream`
  (`crates/upstream`), `agentclient` (`crates/client`), binary `agentcli`
  (`crates/cli`). Origin: `Egor-Liadsky/agent-cli`.
- `agent-sever` — service `agentd` (axum, SQLite via `sqlx`); pulls
  `agentcore` and `agentupstream` as git deps from `agent-cli` `main`.
  Origin: `Egor-Liadsky/agent-server`. The dir name typo is the real path —
  don't fix it.

Tools: git, cargo, gh (only for PRs). Run every command from the specific
repo dir (`git -C <dir>` or `cd <dir> && …`): there is no shared workspace,
`cargo` fails in the root.

## Hard rules (break only with explicit user consent)

- Stage files by path: `git add <path>`. No `git add -A`, `git add .`,
  `git add -u`, `git commit -a` — they pull in unrelated edits. Exception:
  `git add agent-cli agent-sever` in the root stages submodule pointers.
- No `push --force`, `--force-with-lease`, `rebase`, `commit --amend`,
  `reset --hard`.
- Don't edit code to make checks pass. If `cargo build` or `cargo test`
  fails, don't commit in that repo; stop and report.
- Don't run `cargo fmt` on the whole workspace: the code isn't fully
  formatted, so it would touch unrelated files.
- Never commit: `target/`, `logs/`, `agentd.db*`, `.env`,
  `agent-sever/invariants.toml`, `agent-sever/.cargo/config.toml`,
  `openspec-prompt-*.md`, any file with API keys (`sk-…`), tokens or
  passwords. If one shows in `git status`, leave it and name it in the report.
- Don't commit `agent-sever/Cargo.lock` if `agentcore` has no `source` line
  (locked to a path by local `[patch]`): CI and the image build with
  `--locked` and would diverge. Rebuild it via step 7 or leave it and report.
- Commit messages and PR bodies in English only. Russian comments and docs
  inside code are fine.

## Goal

- Each changed submodule: changes split into commits, one logical change
  per commit, unrelated edits not mixed.
- Each changed submodule's `main` pushed to origin.
- If `crates/core` or `crates/upstream` changed and the service uses it:
  `agent-sever/Cargo.lock` points to the pushed `agent-cli` commit, and
  `cargo test --locked` passes in the service.
- Umbrella repo: new submodule pointers (plus root edits, if any) in a
  separate commit, pushed.
- `git status --short` in all three repos is clean except files left on
  purpose and named in the report.

If the user asked for a PR: commits go to a submodule branch, branch pushed,
PR opened to `main`. Don't update the root pointer until the PR is merged —
the root would point to a commit not in `main`.

Source of truth: `/Users/egor_lyadskiy/ai/CLAUDE.md` (core change order,
commit language, README updated with behavior) and active OpenSpec changes in
`openspec/changes/<name>/` of the repo that holds the code. If the diff
breaks them (e.g. `ratatui` or `clap` in `crates/core`, changed `AGENTD_*`
var or hotkey without a README update), report it before committing; the
user decides.

## Steps

1. **Read.** In root, `agent-cli`, `agent-sever` run
   `git branch --show-current`, `git status --short`, `git log --oneline -10`,
   `git diff`, `git diff --staged`. Read `proposal.md` and `tasks.md` of
   unarchived changes in `openspec/changes/` to know the task and its name.
   Stage nothing until the whole diff is read: write messages from content,
   not file names.

2. **Plan.** Group changes by meaning: one group = one commit = one repo.
   For each group list repo, commit title, files. Separately list files that
   must not be committed and conflicts with `CLAUDE.md`. Code, tests and
   README of one change are one group. OpenSpec artifacts go with the code or
   in a separate commit like `Archive <change>, sync <capability> spec`.
   Don't create files or change working-tree content.

3. **Branch.** The project works directly in `main`. On `main` — stay. On
   detached HEAD (common after `git submodule update`) — stop and report: a
   commit there gets lost. If the user asked for a PR, run
   `git switch -c <slug>`, slug = 2–4 English words with hyphens
   (`fix-task-state-clarification-stall`).

4. **Check.** Before the first commit in a repo with changed `.rs`,
   `Cargo.toml` or `Cargo.lock`, run `cargo build` and `cargo test` from its
   dir. Both must pass. On failure: don't commit there, quote the shortest
   decisive line (`error[E0308]: mismatched types` with path and line, or
   `test tests::chat_rejects_api_key ... FAILED` with the first panic line),
   stop, don't claim success.

   Non-blocking: run `rustfmt --check --edition 2024` on changed `.rs` files
   and list files it would rewrite. Don't use clippy as a check: the service
   has ~200 warnings, new ones get lost.

   Changes only in `.md`, `openspec/`, `docs/` need no build — report the
   check as skipped for that reason.

5. **Commit submodules.** `agent-cli` first, then `agent-sever` (service may
   depend on new core). Per group: `git add` with explicit paths,
   `git status --short` to verify the index, `git commit -F -` with heredoc.
   Format as in project history, no Conventional Commits prefixes:

   ```
   Fix task state stalling on the clarification stage

   Body: 1–3 short paragraphs on why the change was needed and what was
   decided, not a diff retelling. Lines ≤72 chars. Skip body only for
   trivial edits.

   Co-Authored-By: <session attribution line, if set>
   ```

   Title: English, imperative, capitalized (`Add`, `Fix`, `Align`,
   `Document`, `Archive`), ≤72 chars, no trailing period.

6. **Push submodules.** `git -C agent-cli push origin main`, then
   `agent-sever`. If rejected as diverged — no rebase, no force; stop and
   show `git log --oneline main..origin/main`.

7. **Service lockfile after core change.** Only if this run committed
   `crates/core` or `crates/upstream` changes the service uses. Local service
   builds use the core working copy via `[patch]` in
   `agent-sever/.cargo/config.toml`; image and CI use pushed `main`. Without
   this step they diverge.

   1. Rename `agent-sever/.cargo/config.toml` to `config.toml.off` (it's
      gitignored).
   2. `cargo update -p agentcore -p agentupstream` — moves only these two;
      `cargo generate-lockfile` would bump everything.
   3. `cargo test --locked`.
   4. Rename the file back, even if the test failed.
   5. `grep -A2 'name = "agentcore"' Cargo.lock` must show
      `source = "git+https://github.com/Egor-Liadsky/agent-cli?branch=main#<sha>"`
      with the just-pushed commit. No `source` line means a path lock — don't
      commit it.
   6. Commit `Cargo.lock` alone: `Bump agentcore/agentupstream lockfile to
      latest main`. Push as in step 6.

8. **Umbrella repo.** After submodule pushes, in root `git add` the changed
   pointers (`agent-cli`, `agent-sever`) and root edits (`CLAUDE.md`,
   `openspec/…`, `.claude/…`) by path. Message as in history:
   `Update agent-sever submodule for <topic>` or
   `Update agent-cli and agent-sever submodules for <topic>`. Then
   `git push origin main`. Don't commit stray untracked root files (e.g.
   `agent-architecture.html`) unless asked — name them in the report.

9. **PR, only on request.** Instead of steps 6–8: `git push -u origin
   <branch>` and `gh pr create --base main --head <branch> --title "<title>"
   --body …`, body in English: Summary (what changed), Testing (commands run
   and results). If `gh` isn't authenticated, still push, and report the link
   `https://github.com/Egor-Liadsky/<agent-cli|agent-server>/pull/new/<branch>`
   plus the exact `gh` error.

10. **Verify.** `git log --oneline -n <commit count>` and
    `git status --short` in all three repos: only files excluded in step 2
    may remain uncommitted.

## Report (in the user's language)

1. Done: commits per repo, one line each, by meaning not file list; whether
   the service lockfile and root pointers were updated.
2. Checks, as run: result of `cargo build`, `cargo test` and (step 7)
   `cargo test --locked` per repo; skipped commands named with reason. Files
   `rustfmt` would rewrite. Result of each `git push`, PR link if any.
3. Decisions and deferred items: how commits were split and why, what was
   left uncommitted and why, conflicts with `CLAUDE.md` and OpenSpec (README
   not updated, terminal crate in core).
4. One concrete next step, e.g. archive the finished OpenSpec change.
