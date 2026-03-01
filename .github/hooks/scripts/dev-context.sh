#!/usr/bin/env bash
# Copyright (c) Vigo Contributors
# SPDX-License-Identifier: MPL-2.0
#
# Vigo dev-session hook -- runs at SessionStart (Linux / macOS CI).
# Reads TASKS.md to detect the current phase, then injects a focused
# system message that sets the agent''s mission and workflow rules.

set -euo pipefail

TASKS="TASKS.md"
SESSION_LOG="SESSION_LOG.md"

# Defaults (fallback if TASKS.md is missing)
CURRENT_PHASE="7"
CURRENT_NAME="JavaScript Engine"
NEXT_PHASE="8"

# Detect current phase.
# Strategy: count done/todo tasks per phase.
# Current = first phase where todo > 0 AND done == 0.
if [[ -f "$TASKS" ]]; then
    declare -A phase_done
    declare -A phase_todo
    declare -A phase_name
    cur_phase=""

    while IFS= read -r line; do
        # Match: ## Phase 7 -- JavaScript Engine
        if [[ "$line" =~ ^##[[:space:]]+Phase[[:space:]]+([0-9]+)[^0-9](.*) ]]; then
            cur_phase="${BASH_REMATCH[1]}"
            raw_name="${BASH_REMATCH[2]}"
            # Strip leading dashes / em-dashes / spaces
            raw_name="${raw_name##*( )}"
            raw_name="${raw_name#*--}"
            raw_name="${raw_name#*-}"
            raw_name="${raw_name## }"
            phase_name["$cur_phase"]="${raw_name}"
            phase_done["$cur_phase"]="0"
            phase_todo["$cur_phase"]="0"
        fi
        if [[ -n "$cur_phase" ]]; then
            # U+2705 = checkmark (done), U+2B1C = white-square (todo)
            if [[ "$line" == *$'\xe2\x9c\x85'* ]]; then
                phase_done["$cur_phase"]=$(( ${phase_done[$cur_phase]:-0} + 1 ))
            fi
            if [[ "$line" == *$'\xe2\xac\x9c'* ]]; then
                phase_todo["$cur_phase"]=$(( ${phase_todo[$cur_phase]:-0} + 1 ))
            fi
        fi
    done < "$TASKS"

    # Find first phase where todo > 0 and done == 0
    for pn in $(echo "${!phase_done[@]}" | tr ' ' '\n' | sort -n); do
        done_c="${phase_done[$pn]:-0}"
        todo_c="${phase_todo[$pn]:-0}"
        if (( todo_c > 0 && done_c == 0 )); then
            CURRENT_PHASE="$pn"
            CURRENT_NAME="${phase_name[$pn]}"
            NEXT_PHASE=$(( pn + 1 ))
            break
        fi
    done
fi

# Extract current phase task block (first 80 lines of that section)
PHASE_BLOCK="(phase block unavailable)"
if [[ -f "$TASKS" ]]; then
    PHASE_BLOCK=$(
        awk "
            /^## Phase ${CURRENT_PHASE}[^0-9]/ { p=1 }
            p && /^## Phase [0-9]/ && !/^## Phase ${CURRENT_PHASE}[^0-9]/ { p=0 }
            p { print }
        " "$TASKS" | head -80
    )
fi

# Recent session log
LOG_SNIP=""
if [[ -f "$SESSION_LOG" ]]; then
    LOG_SNIP=$(tail -30 "$SESSION_LOG")
fi

# Build message (ASCII-clean)
MSG="================================================================
  VIGO DEV SESSION -- AUTO-INJECTED CONTEXT
  Phase ${CURRENT_PHASE}: ${CURRENT_NAME}  (CURRENT)
  Next:  Phase ${NEXT_PHASE}
================================================================

## Your Mission

Continue the development. Make use of all the tools that you can.
Use sub agents. Do not overcode. Follow the instructions. Make sure
everything you code works properly -- not just the tests. Run them,
check for errors, double check the code. Be creative and mindful.

## Mandatory Session Workflow

1. ORIENT -- Read PLAN.md (Phase ${CURRENT_PHASE} section) and TASKS.md
   before writing any code. Use a sub-agent to explore existing code
   when you need context.

2. WORK TASK BY TASK -- for every Phase ${CURRENT_PHASE} task:
   - Write the code.
   - Run: just build   (fix errors before moving on)
   - Run: just test    (fix any failures)
   - Run: just lint    (fix ALL Clippy warnings -- zero allowed)
   - Mark the task done in TASKS.md immediately.

3. USE SUB-AGENTS -- use sub-agents for codebase exploration,
   pattern verification, and validation to keep this context focused.

4. VERIFY RUNTIME BEHAVIOR -- do not only check that tests pass.
   Run the binary where possible. Check for panics and wrong output.

5. PHASE COMPLETION -- when every Phase ${CURRENT_PHASE} task is done:
   a. Run:  just ci
      (must exit 0: fmt-check + clippy + test)
   b. Update SESSION_LOG.md -- add a phase summary at the top.
   c. Update TASKS.md       -- mark all Phase ${CURRENT_PHASE} tasks done.
   d. Commit:
        git add -A
        git commit -m \"Phase ${CURRENT_PHASE} complete: ${CURRENT_NAME}\"
   e. Announce completion and ask to start Phase ${NEXT_PHASE}.

## Non-Negotiable Rules

- Zero Clippy warnings (cargo clippy --workspace -- -D warnings).
- No .unwrap() / .expect() in library crates -- use ? operator.
- No println! / eprintln! in library crates -- use tracing:: macros.
- Every unsafe block must have a // SAFETY: comment.
- Every .rs and .zig file must start with the MPL-2.0 header.
- All crate deps declared in [workspace.dependencies] in root Cargo.toml.
- Use thiserror in library crates; anyhow only in vex-app.
- Only implement Phase ${CURRENT_PHASE} tasks -- no speculative features.
- Every public function added must have at least one test.

## Key Reference Files

  PLAN.md              -- master plan (phases + deliverables)
  TASKS.md             -- task checklist (update as you complete tasks)
  SESSION_LOG.md       -- session history (update after each phase)
  .github/copilot-instructions.md
  .github/instructions/vex-coding-conventions.instructions.md
  .github/instructions/vex-zig-build.instructions.md
  .github/instructions/vex-render-apis.instructions.md
  docs/ARCHITECTURE.md  docs/RUST_STYLE.md  docs/ZIG_STYLE.md
  docs/FFI_CONVENTIONS.md

## Recent Session Log (last 30 lines)

${LOG_SNIP}

## Phase ${CURRENT_PHASE} Task Block (from TASKS.md)

${PHASE_BLOCK}"

# Output JSON using python3 for safe encoding
python3 -c "
import json, sys
msg = sys.stdin.read()
print(json.dumps({'systemMessage': msg}))
" <<< "$MSG"
