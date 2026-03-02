# Copyright (c) Vigo Contributors
# SPDX-License-Identifier: MPL-2.0
#
# Vigo dev-session hook -- runs at SessionStart.
# Reads FINAL_TASKS.md, finds the current section (first with unchecked tasks),
# and injects a focused system message with the active task list.

param()
Set-StrictMode -Version Latest
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# ── Read project files ────────────────────────────────────────────────────────
$tasksRaw  = Get-Content "FINAL_TASKS.md" -Raw -Encoding UTF8 -ErrorAction SilentlyContinue
$logSnip   = if (Test-Path "SESSION_LOG.md") {
    (Get-Content "SESSION_LOG.md" -Encoding UTF8 | Select-Object -Last 30) -join "`n"
} else { "(SESSION_LOG.md not found)" }

# ── Parse sections and checkbox counts ───────────────────────────────────────
# Section headings look like:  ## Foundation & Config (Tasks 1-3)
# or plain ## headings for non-task sections (Key Decisions, Success Criteria)
# We only track headings that immediately precede checkbox lines.

$sections = [System.Collections.Generic.List[hashtable]]::new()
$curSection = $null

if ($tasksRaw) {
    foreach ($line in ($tasksRaw -split "`n")) {
        # Match section header that has a task range in parens, or any ## heading
        if ($line -match '^##\s+(.+)') {
            # Save previous section if it had tasks
            if ($null -ne $curSection -and ($curSection.done + $curSection.todo) -gt 0) {
                $sections.Add($curSection)
            }
            $curSection = @{ name = $Matches[1].Trim(); done = 0; todo = 0; tasks = [System.Collections.Generic.List[string]]::new() }
        }
        if ($null -ne $curSection) {
            if ($line -match '^\s*-\s+\[x\]') {
                $curSection.done++
            } elseif ($line -match '^\s*-\s+\[ \]') {
                $curSection.todo++
                # Extract task description (bold text between **)
                $desc = $line -replace '^\s*-\s+\[ \]\s+\*\*', '' -replace '\*\*.*$', ''
                $curSection.tasks.Add($desc.Trim())
            }
        }
    }
    # Save last section
    if ($null -ne $curSection -and ($curSection.done + $curSection.todo) -gt 0) {
        $sections.Add($curSection)
    }
}

# ── Find current section (first with unchecked tasks) ────────────────────────
$total       = 0
$totalDone   = 0
$curName     = "Foundation & Config"
$curTasks    = @()
$nextName    = ""
$foundCur    = $false

foreach ($s in $sections) {
    $total    += $s.done + $s.todo
    $totalDone+= $s.done
    if (-not $foundCur -and $s.todo -gt 0) {
        $curName  = $s.name
        $curTasks = $s.tasks.ToArray()
        $foundCur = $true
    } elseif ($foundCur -and $nextName -eq "" -and $s.todo -gt 0) {
        $nextName = $s.name
    }
}

$totalRemaining = $total - $totalDone
$progressPct    = if ($total -gt 0) { [int](($totalDone / $total) * 100) } else { 0 }

# ── Build next-tasks preview (up to 8 items) ─────────────────────────────────
$taskPreview = if ($curTasks.Count -gt 0) {
    ($curTasks | Select-Object -First 8 | ForEach-Object { "  - [ ] $_" }) -join "`n"
} else { "  (all tasks in this section complete)" }

# ── Build system message ──────────────────────────────────────────────────────
$msg = @"
================================================================
  VIGO FINAL ASSEMBLY -- AUTO-INJECTED CONTEXT
  Task list: FINAL_TASKS.md (77 wiring tasks total)
  Progress:  ${totalDone}/${total} done (${progressPct}%)
  Remaining: ${totalRemaining} tasks
================================================================

  Current section: ${curName}
  Next section:    $(if ($nextName) { $nextName } else { "(this is the last section)" })

  Next tasks to work on:
${taskPreview}

================================================================

## Your Mission

Continue the development. Make use of all the tools that you can.
Use sub agents. Do not overcode. Follow all the instructions,
plans, and hooks. Make sure everything you code works properly --
not just the tests. Run them, check for errors, double check the
code. Be creative and mindful.

## Mandatory Workflow

1. ORIENT -- Use sub-agents to explore the relevant crates and
   files listed in the task before writing a single line.
   Read FINAL_TASKS.md for the task description and file list.

2. WORK TASK BY TASK -- for each unchecked task in the current
   section:
   - Read the task description + file list carefully.
   - Explore the existing stubs/code with a sub-agent.
   - Wire it up. Do not rewrite working code unnecessarily.
   - Run: just build   (fix errors before moving on)
   - Run: just test    (fix any failures)
   - Run: just lint    (zero Clippy warnings required)
   - Mark done in FINAL_TASKS.md:  - [ ]  -->  - [x]

3. USE SUB-AGENTS -- for codebase exploration, pattern checks,
   and validation steps, to keep this context focused.

4. VERIFY THE WIRING WORKS -- "wired" means actually connected
   and exercised at runtime, not just that the code compiles.
   Run the browser when possible. Check real behavior.

5. SECTION COMPLETE -- when all tasks in the current section
   are marked [x]:
   a. Run:  just ci   (must exit 0)
   b. Update SESSION_LOG.md with a section summary.
   c. Commit:  git add -A
               git commit -m "Wire: <section name>"
   d. Announce and continue to the next section.

## Non-Negotiable Rules

- Zero Clippy warnings (cargo clippy --workspace -- -D warnings).
- No .unwrap() / .expect() in library crates -- use ? operator.
- No println! / eprintln! in library crates -- use tracing::.
- Every unsafe block must have a // SAFETY: comment.
- Every .rs / .zig file must start with the MPL-2.0 header.
- All deps declared in [workspace.dependencies] in root Cargo.toml.
- Do not add new features -- only wire up what already exists.
- Wired functions where the logic is non-trivial must have tests.

## Key Reference Files

  FINAL_TASKS.md       -- the 77 wiring tasks (UPDATE as you go)
  SESSION_LOG.md       -- session history
  .github/copilot-instructions.md
  .github/instructions/vex-coding-conventions.instructions.md
  .github/instructions/vex-zig-build.instructions.md
  .github/instructions/vex-render-apis.instructions.md
  docs/ARCHITECTURE.md  docs/RUST_STYLE.md  docs/ZIG_STYLE.md

## Success Criteria (from FINAL_TASKS.md)

  1. just ci passes clean
  2. just run opens window, loads vex://welcome, renders
  3. Type URL -> page loads, renders, JS executes
  4. Click links -> navigation, back/forward work
  5. Ctrl+T/W -> tabs open/close
  6. console.log() appears in DevTools console
  7. <input> elements accept text
  8. localStorage persists across page loads
  9. Images load asynchronously
  10. Find-in-page (Ctrl+F) highlights matches

## Recent Session Log (last 30 lines)

${logSnip}
"@

[PSCustomObject]@{ systemMessage = $msg } | ConvertTo-Json -Compress -Depth 3
