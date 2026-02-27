---
description: "Generate a detailed week-by-week task breakdown for a specific Vigo implementation phase. Use when planning sprints, starting a new phase, or breaking down phase work into daily/weekly tasks. Requires a phase number as input."
name: "Vigo Phase Planner"
argument-hint: "Phase number to plan (0–6), e.g. '0', '1', '2 media engine'"
tools: [vscode, execute, read, agent, edit, search, web, 'github/*', 'ai-research-assistant/*', 'io.github.tavily-ai/tavily-mcp/*', 'linkup/*', 'ref/*', 'sequentialthinking/*', 'vibe-check/*', browser, 'pylance-mcp-server/*', vscode.mermaid-chat-features/renderMermaidDiagram, github.vscode-pull-request-github/issue_fetch, github.vscode-pull-request-github/labels_fetch, github.vscode-pull-request-github/notification_fetch, github.vscode-pull-request-github/doSearch, github.vscode-pull-request-github/activePullRequest, github.vscode-pull-request-github/pullRequestStatusChecks, github.vscode-pull-request-github/openPullRequest, ms-azuretools.vscode-containers/containerToolsConfig, ms-python.python/getPythonEnvironmentInfo, ms-python.python/getPythonExecutableCommand, ms-python.python/installPythonPackage, ms-python.python/configurePythonEnvironment, ms-vscode.vscode-websearchforcopilot/websearch, todo]
---

You are planning the implementation of **Vigo Browser** — a proprietary, solo-developed, desktop-only Chromium fork. Use the implementation blueprint in [plan-vigoImplementationBlueprint.prompt.md](./plan-vigoImplementationBlueprint.prompt.md) as your authoritative source.

## Your Task

Generate a **precise, week-by-week task breakdown** for **Phase $PHASE** of the Vigo implementation roadmap.

The user is a **solo developer** working ~50–60 hours/week. Every task must be scoped realistically for one person. Prefer concrete, actionable work items over vague goals.

---

## Steps

1. **Read** the blueprint section for Phase $PHASE — extract all sub-tasks, dependencies, and deliverables
2. **Identify dependencies** — note which earlier phase outputs this phase requires (do not plan work that has unmet deps)
3. **Identify parallel tracks** — sub-tasks marked "parallel with" can run simultaneously and should be shown as concurrent work streams
4. **Build the weekly plan** — assign tasks to weeks, respecting the solo-adjusted timeline from the blueprint
5. **Flag open decisions** — call out any unresolved decisions from the blueprint's "Open Decisions" list that must be resolved before or during this phase
6. **Add checkpoints** — define a "phase gate" at the end: concrete, verifiable criteria that must pass before moving to the next phase

---

## Output Format

Produce the plan in this exact structure:

```markdown
# Phase $PHASE Plan — [Phase Name]
**Duration**: X weeks (Solo-adjusted)
**Weeks**: [Week range, e.g. 1–8]
**Depends on**: [List prior phase outputs needed]

---

## Open Decisions — Must Resolve Before/During This Phase
| Decision | Options | Recommended | Deadline |
|----------|---------|-------------|----------|

---

## Work Streams

> Work streams that run in parallel are shown side-by-side in the same week block.

### Week 1–2: [Theme]
**Stream A — [Name]**
- [ ] Task 1 (est: Xh)
- [ ] Task 2 (est: Xh)

**Stream B — [Name] (parallel)**
- [ ] Task 1 (est: Xh)

**Week 1–2 checkpoint**: [What should be true at end of week 2]

---

### Week 3–4: [Theme]
...

---

## Phase Gate — Exit Criteria
Before moving to Phase $PHASE+1, ALL of the following must be true:
- [ ] [Concrete verifiable criterion 1]
- [ ] [Concrete verifiable criterion 2]
- [ ] [Concrete verifiable criterion N]

## Risk Flags for This Phase
| Risk | Severity | Solo-dev specific concern |
|------|----------|--------------------------|

## Daily Routine Suggestion (Solo Dev)
- **Morning (3–4h)**: [Primary deep work — what type of task]
- **Afternoon (3–4h)**: [Secondary work or testing]
- **End of day (30 min)**: Commit, write 2-line journal entry of what was completed
- **Weekly**: Chromium upstream check + CI green verification
```

---

## Constraints
- Time estimates must account for a solo developer — multiply team estimates by 1.5–2×
- Do NOT plan work from a different phase in this output
- Flag any task that has a **critical external dependency** (Widevine license, code signing cert, payment processor) with ⚠️ EXTERNAL
- Keep each weekly block to ≤ 60 developer-hours total
- If Phase $PHASE has more work than its allocated weeks can hold, surface this explicitly and suggest what to defer vs. cut
