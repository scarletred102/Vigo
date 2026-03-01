# Copyright (c) Vigo Contributors
# SPDX-License-Identifier: MPL-2.0
#
# Vigo dev-session hook -- runs at SessionStart.
# Reads TASKS.md to detect the current phase, then injects a focused
# system message that sets the agent''s mission and workflow rules.

param()

Set-StrictMode -Version Latest
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# Read project files
$tasksRaw = Get-Content "TASKS.md" -Raw -Encoding UTF8 -ErrorAction SilentlyContinue
$logSnip  = ""
if (Test-Path "SESSION_LOG.md") {
    $logSnip = (Get-Content "SESSION_LOG.md" -Encoding UTF8 | Select-Object -Last 30) -join "`n"
}

# Detect current phase.
# Strategy: count done/todo tasks per phase using [x] and [ ] markers.
# Current phase = first phase with todo > 0 AND done == 0
# (a phase that has not been started yet -- that is where we work next).
# Partially-done phases (both done + todo) are stragglers to deprioritise.
$currentPhase     = "7"
$currentPhaseName = "JavaScript Engine"
$nextPhase        = "8"

if ($tasksRaw) {
    $phaseMap = [System.Collections.Generic.SortedDictionary[int,hashtable]]::new()
    $curPN    = 0

    foreach ($line in ($tasksRaw -split "`n")) {
        if ($line -match '^##\s+Phase\s+(\d+)[^0-9](.*)') {
            $curPN   = [int]$Matches[1]
            $rawName = ($Matches[2] -replace '^[\s\u2014\-]+', '').Trim()
            if (-not $phaseMap.ContainsKey($curPN)) {
                $phaseMap[$curPN] = @{ name = $rawName; done = 0; todo = 0 }
            }
        }
        if ($curPN -gt 0) {
            # Status column uses Unicode: checkmark = U+2705, white-square = U+2B1C
            if ($line -match "\u2705") { $phaseMap[$curPN].done++ }
            if ($line -match "\u2B1C") { $phaseMap[$curPN].todo++ }
        }
    }

    # First phase with todo > 0 and no done tasks = the current phase to work on
    foreach ($pn in $phaseMap.Keys) {
        $e = $phaseMap[$pn]
        if ($e.todo -gt 0 -and $e.done -eq 0) {
            $currentPhase     = "$pn"
            $currentPhaseName = $e.name
            $nextPhase        = [string]($pn + 1)
            break
        }
    }
}

# Extract current phase task block from TASKS.md (capped at 4000 chars)
$phaseBlock = "(phase block unavailable)"
if ($tasksRaw) {
    $esc     = [regex]::Escape($currentPhase)
    $pattern = "(?s)(##\s+Phase\s+${esc}[^\d].*?)(?=\n##\s+Phase\s+\d+|\z)"
    $m       = [regex]::Match($tasksRaw, $pattern)
    if ($m.Success) {
        $raw        = $m.Value
        $phaseBlock = if ($raw.Length -gt 4000) { $raw.Substring(0, 4000) + "`n...(truncated)" } else { $raw }
    }
}

# Build system message (fully ASCII to avoid encoding hazards)
$msg = @"
================================================================
  VIGO DEV SESSION -- AUTO-INJECTED CONTEXT
  Phase ${currentPhase}: ${currentPhaseName}  (CURRENT)
  Next:  Phase ${nextPhase}
================================================================

## Your Mission

Continue the development. Make use of all the tools that you can.
Use sub agents. Do not overcode. Follow the instructions. Make sure
everything you code works properly -- not just the tests. Run them,
check for errors, double check the code. Be creative and mindful.

## Mandatory Session Workflow

1. ORIENT -- Read PLAN.md (Phase ${currentPhase} section) and TASKS.md
   before writing any code. Use a sub-agent to explore existing code
   when you need context.

2. WORK TASK BY TASK -- for every Phase ${currentPhase} task:
   - Write the code.
   - Run: just build   (fix errors before moving on)
   - Run: just test    (fix any failures)
   - Run: just lint    (fix ALL Clippy warnings -- zero allowed)
   - Mark the task done in TASKS.md immediately.

3. USE SUB-AGENTS -- use sub-agents for codebase exploration,
   pattern verification, and validation to keep this context focused.

4. VERIFY RUNTIME BEHAVIOR -- do not only check that tests pass.
   Run the binary where possible. Check for panics, wrong output,
   and runtime errors.

5. PHASE COMPLETION -- when every Phase ${currentPhase} task is done:
   a. Run:  just ci
      (must exit 0: fmt-check + clippy + test)
   b. Update SESSION_LOG.md -- add a phase summary at the top.
   c. Update TASKS.md       -- mark all Phase ${currentPhase} tasks done.
   d. Commit:
        git add -A
        git commit -m "Phase ${currentPhase} complete: ${currentPhaseName}"
   e. Announce completion and ask to start Phase ${nextPhase}.

## Non-Negotiable Rules

- Zero Clippy warnings (cargo clippy --workspace -- -D warnings).
- No .unwrap() / .expect() in library crates -- use ? operator.
- No println! / eprintln! in library crates -- use tracing:: macros.
- Every unsafe block must have a // SAFETY: comment.
- Every .rs and .zig file must start with the MPL-2.0 header.
- All crate deps declared in [workspace.dependencies] in root Cargo.toml.
  Do not add a dep without checking for duplicates first.
- Use thiserror in library crates; anyhow only in vex-app.
- Only implement Phase ${currentPhase} tasks -- no speculative features.
- Every public function added must have at least one test.

## Key Reference Files

  PLAN.md              -- master plan (phases + deliverables)
  TASKS.md             -- task checklist (update as you complete tasks)
  SESSION_LOG.md       -- session history (update after each phase)
  .github/copilot-instructions.md         -- workspace-wide rules
  .github/instructions/vex-coding-conventions.instructions.md
  .github/instructions/vex-zig-build.instructions.md
  .github/instructions/vex-render-apis.instructions.md
  docs/ARCHITECTURE.md  docs/RUST_STYLE.md  docs/ZIG_STYLE.md
  docs/FFI_CONVENTIONS.md

## Recent Session Log (last 30 lines of SESSION_LOG.md)

${logSnip}

## Phase ${currentPhase} Task Block (from TASKS.md)

${phaseBlock}
"@

[PSCustomObject]@{ systemMessage = $msg } | ConvertTo-Json -Compress -Depth 3
