---
name: "Vex Continue Dev"
description: "Continue Vigo/Vex final browser assembly. Reads FINAL_TASKS.md, finds the current active section, and works task-by-task until the section is wired, tested, and committed. Optional argument: section name or task number to jump to."
argument-hint: "Optional: section name or task number (e.g. 'Network' or '4'). Defaults to first incomplete section."
agent: agent
tools: [read, edit, search, execute, todo, agent]
---

# Vex Final Assembly — Continue Development

## Context to read first (do this before anything else)

Read ALL of the following files in parallel using sub-agents. You must have this context before writing a single line of code:

- [FINAL_TASKS.md](../../FINAL_TASKS.md) — the 77 wiring tasks; find the current section (first with unchecked `- [ ]` items) and read it fully
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

## Pre-Flight Checklist

Before touching any task:

1. Run `just ci` and confirm it exits 0 (fmt-check + clippy + test all pass)
2. Read FINAL_TASKS.md and identify the **current section** — the first `## Section` heading that still has unchecked `- [ ]` items
3. If `$input` was provided, jump to the section matching that name or task number instead
4. Read SESSION_LOG.md to understand the most recent context
5. If CI fails, fix it before wiring anything new — do not accumulate broken builds

## Implementation Workflow

Work task-by-task. For each task in Phase $input:

### Step 1 — Orient
- Use a sub-agent to explore the relevant existing crates and files
- Understand patterns already established (naming, error types, test conventions)
- Read the exact PLAN.md rationale for this task before designing it

### Step 2 — Plan
- Each task in FINAL_TASKS.md lists the specific files to touch/wire
- When a new file is required, add MPL-2.0 headers to every new `.rs` and `.zig` file
- Define the connection point (function call, trait impl, or FFI bridge) before writing the body

### Step 3 — Wire
- Connect the existing stubs to the existing callers; avoid rewriting working code
- Follow conventions from the instruction files
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
- Update the task in FINAL_TASKS.md from `- [ ]` to `- [x]`
- Add a one-line note to SESSION_LOG.md under the current section heading

### Step 6 — Next task
- Repeat from Step 1 for the next `- [ ]` task in the current section

## Section Completion

When every task in the current section is marked `- [x]`:

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

3. Update SESSION_LOG.md — add a section summary block at the very top:
   ```markdown
   ## <Section Name> (YYYY-MM-DD)
   - <wired connection 1>
   - <wired connection 2>
   - Test count: XXX Rust, YY Zig
   - Tasks: N/77 total done
   ```

4. Confirm every task in this section is `- [x]` in FINAL_TASKS.md

5. Commit:
   ```
   git add -A
   git commit -m "Wire: <Section Name>"
   ```

6. Announce section completion with a summary, then continue to the next section

## Non-Negotiable Rules (summary)

| Rule | Enforced by |
|------|------------|
| Zero Clippy warnings | `just lint` / CI |
| No `.unwrap()` in lib crates | Code review / reviewer agent |
| No `println!` in lib crates | Code review |
| MPL-2.0 header on every file | Code review |
| Only wire existing code — no new features | Task scope |
| All deps via workspace deps | Code review |
| `thiserror` in libs, `anyhow` in vex-app | Code review |

## Reference Summary

| File | What to use it for |
|------|-------------------|
| FINAL_TASKS.md | The 77 wiring tasks — update `- [ ]` → `- [x]` as you go |
| SESSION_LOG.md | Prior art, established patterns, test baselines |
| docs/ARCHITECTURE.md | Crate dependency graph — where does new code connect? |
| .github/instructions/*.instructions.md | Style, safety, API patterns |
