#!/usr/bin/env bash
# Copyright (c) Vigo Contributors
# SPDX-License-Identifier: MPL-2.0
#
# Vigo dev-session hook -- runs at SessionStart (Linux/macOS).
# Reads FINAL_TASKS.md, finds the current section (first with unchecked tasks),
# and injects a focused system message.

set -euo pipefail

TASKS="FINAL_TASKS.md"
SESSION_LOG="SESSION_LOG.md"

# ---- parse checkbox counts per section ------------------------------------
declare -a SEC_NAMES=()
declare -a SEC_DONE=()
declare -a SEC_TODO=()
declare -A SEC_TASKS_MAP=()   # section_index -> newline-separated task titles

current_sec=""
sec_idx=-1

while IFS= read -r line; do
  if [[ "$line" =~ ^##[[:space:]]+(.+) ]]; then
    current_sec="${BASH_REMATCH[1]}"
    # Will finalise when we see the first checkbox or next heading
    sec_idx=$(( sec_idx + 1 ))
    SEC_NAMES[$sec_idx]="$current_sec"
    SEC_DONE[$sec_idx]=0
    SEC_TODO[$sec_idx]=0
    SEC_TASKS_MAP[$sec_idx]=""
  fi
  if [[ $sec_idx -ge 0 ]]; then
    if [[ "$line" =~ ^[[:space:]]*-[[:space:]]+'\[x\]' ]]; then
      SEC_DONE[$sec_idx]=$(( SEC_DONE[$sec_idx] + 1 ))
    elif [[ "$line" =~ ^[[:space:]]*-[[:space:]]+'[ ]' ]]; then
      SEC_TODO[$sec_idx]=$(( SEC_TODO[$sec_idx] + 1 ))
      # extract task description between **…** (first ** to second **)
      desc=$(echo "$line" | sed "s/^[[:space:]]*-[[:space:]]*\[ \][[:space:]]*\*\*//;s/\*\*.*$//")
      if [[ -n "${SEC_TASKS_MAP[$sec_idx]}" ]]; then
        SEC_TASKS_MAP[$sec_idx]="${SEC_TASKS_MAP[$sec_idx]}
  - [ ] $desc"
      else
        SEC_TASKS_MAP[$sec_idx]="  - [ ] $desc"
      fi
    fi
  fi
done < "$TASKS"

# ---- totals and find active section --------------------------------------
total=0
total_done=0
cur_name=""
cur_tasks=""
next_name=""
found_cur=false

for i in "${!SEC_NAMES[@]}"; do
  s_done="${SEC_DONE[$i]}"
  s_todo="${SEC_TODO[$i]}"
  s_total=$(( s_done + s_todo ))
  if [[ $s_total -eq 0 ]]; then continue; fi
  total=$(( total + s_total ))
  total_done=$(( total_done + s_done ))
  if [[ "$found_cur" == "false" && $s_todo -gt 0 ]]; then
    cur_name="${SEC_NAMES[$i]}"
    cur_tasks="${SEC_TASKS_MAP[$i]}"
    # trim to first 8 lines
    cur_tasks=$(echo "$cur_tasks" | head -8)
    found_cur=true
  elif [[ "$found_cur" == "true" && -z "$next_name" && $s_todo -gt 0 ]]; then
    next_name="${SEC_NAMES[$i]}"
  fi
done

remaining=$(( total - total_done ))
if [[ $total -gt 0 ]]; then
  pct=$(( (total_done * 100) / total ))
else
  pct=0
fi
[[ -z "$next_name" ]] && next_name="(this is the last section)"

# ---- session log snippet --------------------------------------------------
if [[ -f "$SESSION_LOG" ]]; then
  log_snip=$(tail -30 "$SESSION_LOG")
else
  log_snip="(SESSION_LOG.md not found)"
fi

# ---- build message --------------------------------------------------------
msg=$(cat <<MSG
================================================================
  VIGO FINAL ASSEMBLY -- AUTO-INJECTED CONTEXT
  Task list: FINAL_TASKS.md (77 wiring tasks total)
  Progress:  ${total_done}/${total} done (${pct}%)
  Remaining: ${remaining} tasks
================================================================

  Current section: ${cur_name}
  Next section:    ${next_name}

  Next tasks to work on:
${cur_tasks}

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

${log_snip}
MSG
)

# ---- emit JSON -----------------------------------------------------------
# Use python3 if available, otherwise jq, otherwise manual escape
if command -v python3 &>/dev/null; then
  echo "$msg" | python3 -c "
import sys, json
print(json.dumps({'systemMessage': sys.stdin.read()}))"
elif command -v jq &>/dev/null; then
  echo "{\"systemMessage\": $(echo "$msg" | jq -Rs .)}"
else
  # Minimal fallback: escape backslash, quote, newline
  escaped=$(echo "$msg" | sed 's/\\/\\\\/g;s/"/\\"/g' | awk '{printf "%s\\n", $0}')
  echo "{\"systemMessage\": \"${escaped}\"}"
fi