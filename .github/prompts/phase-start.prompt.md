---
name: "Vex Phase Start"
description: "Start a new development phase for the Vigo/Vex engine. Reads PLAN.md, TASKS.md, SESSION_LOG.md, all instructions and hooks, then scaffolds and begins implementation."
argument-hint: "Phase number to start (e.g. 7)"
agent: agent
tools: [read, edit, search, execute, todo, agent]
---

# Vex Phase Start — Phase $input

## Context to read first (do this before anything else)

Read ALL of the following files in parallel using sub-agents. You must have this context before writing a single line of code:

- [PLAN.md](../../PLAN.md) — find the `## Phase $input` section and read it fully
- [TASKS.md](../../TASKS.md) — extract every task row for `## Phase $input` and list them
- [SESSION_LOG.md](../../SESSION_LOG.md) — read the most recent session summary to understand what was just completed
- [.github/copilot-instructions.md](../copilot-instructions.md) — workspace rules (always active)
- [.github/instructions/vex-coding-conventions.instructions.md](../instructions/vex-coding-conventions.instructions.md)
- [.github/instructions/vex-zig-build.instructions.md](../instructions/vex-zig-build.instructions.md)
- [.github/instructions/vex-render-apis.instructions.md](../instructions/vex-render-apis.instructions.md)
- [.github/hooks/scripts/dev-context.ps1](../hooks/scripts/dev-context.ps1) — workflow rules baked into the session hook
- [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md)
- [docs/RUST_STYLE.md](../../docs/RUST_STYLE.md)
- [docs/ZIG_STYLE.md](../../docs/ZIG_STYLE.md)
- [docs/FFI_CONVENTIONS.md](../../docs/FFI_CONVENTIONS.md)

## Your Mission

Continue the development. Make use of all the tools that you can. Use sub agents. Don't overcode. Follow all the instructions, plans, hooks and make sure everything you code works properly — not just the tests. Run them, check for errors, double check the code. Be creative and mindful.

## Phase $input — Pre-Flight Checklist

Before writing any Phase $input code, verify the previous phase is complete:

1. Run `just ci` and confirm it exits 0 (fmt-check + clippy + test all pass)
2. Check that every task in Phase `$input - 1` is marked `✅` in TASKS.md
3. Read SESSION_LOG.md to confirm the previous phase summary is written
4. If anything is incomplete, fix it before proceeding — do not start a new phase on a broken foundation

## Implementation Workflow

Work task-by-task. For each task in Phase $input:

### Step 1 — Orient
- Use a sub-agent to explore the relevant existing crates and files
- Understand patterns already established (naming, error types, test conventions)
- Read the exact PLAN.md rationale for this task before designing it

### Step 2 — Scaffold
- Create the files/modules listed in the task's Deliverable column
- Add MPL-2.0 headers to every new `.rs` and `.zig` file
- Define the public API (types + function signatures) first; implement after

### Step 3 — Implement
- Write the implementation following conventions from the instruction files
- No `.unwrap()` / `.expect()` in library code — use `?`
- No `println!` — use `tracing::` macros
- No speculative features — only what the task description says

### Step 4 — Verify (all four must pass)
```
just build    # zero errors
just test     # zero failures
just lint     # zero Clippy warnings (-D warnings)
just run      # binary opens without panic (where applicable)
```

If any step fails, fix it before moving to the next task. Never accumulate broken tasks.

### Step 5 — Mark complete
- Update the task row in TASKS.md from `⬜` to `✅`
- Add a one-line note to SESSION_LOG.md under the current phase heading

### Step 6 — Next task
- Repeat from Step 1 for the next `⬜` task in Phase $input

## Phase Completion

When every Phase $input task is marked `✅`:

1. Run the full CI gate:
   ```
   just ci
   ```
   This runs fmt-check + lint + test. It must exit 0. Fix any failure before continuing.

2. Run the reviewer agent to validate completeness:
   ```
   @vex-reviewer
   ```
   Address every issue the reviewer raises before committing.

3. Update SESSION_LOG.md — add a phase summary block at the very top:
   ```markdown
   ## Phase $input — <Phase Name> ✅ (YYYY-MM-DD)
   - <key deliverable 1>
   - <key deliverable 2>
   - Test count: XXX Rust, YY Zig
   ```

4. Update TASKS.md — every Phase $input task must be `✅`

5. Commit:
   ```
   git add -A
   git commit -m "Phase $input complete: <Phase Name>"
   ```

6. Announce completion with a summary, then ask: **"Ready to start Phase ${{input_plus_one}}?"**

## Non-Negotiable Rules (summary)

| Rule | Enforced by |
|------|------------|
| Zero Clippy warnings | `just lint` / CI |
| No `.unwrap()` in lib crates | Code review / reviewer agent |
| No `println!` in lib crates | Code review |
| MPL-2.0 header on every file | Code review |
| Every public fn has a test | Reviewer agent |
| Only Phase $input scope | This prompt |
| All deps via workspace deps | Code review |
| `thiserror` in libs, `anyhow` in vex-app | Code review |

## Reference Summary

| File | What to use it for |
|------|-------------------|
| PLAN.md §Phase $input | Rationale, architecture decisions, exit criteria |
| TASKS.md §Phase $input | Exact deliverables, file names, test counts |
| SESSION_LOG.md | Prior art, established patterns, test baselines |
| docs/ARCHITECTURE.md | Crate dependency graph — where does new code belong? |
| .github/instructions/*.instructions.md | Style, safety, API patterns |
