# OMEGA Ω

## Philosophy
This project uses OMEGA, a multi-agent workflow designed to produce the highest quality code possible.
Each agent has a specific role and the code passes through multiple validation layers before being considered complete.
Every agent reads from and writes to a shared institutional memory (SQLite) — no agent acts alone, without backpressure.

## Source of Truth Hierarchy
1. **Codebase** — the ultimate source of truth. Always trust code over documentation.
2. **`.claude/memory.db`** — institutional memory. Accumulated decisions, failed approaches, hotspots, findings across all sessions.
3. **specs/** — technical specifications per domain. `specs/SPECS.md` is the master index.
4. **docs/** — user-facing and developer documentation. `docs/DOCS.md` is the master index.

When specs or docs conflict with the codebase, the codebase wins. Agents must flag the discrepancy and update specs/docs accordingly.

## Institutional Memory

Every workflow reads from and writes to `.claude/memory.db`. This is the backpressure mechanism that prevents agents from acting in isolation. **This protocol is not optional** — it is the foundation that gives OMEGA persistent knowledge across sessions.

### DB Detection
At the start of **every session and every workflow**, check if the DB exists:

```bash
test -f .claude/memory.db && echo "DB_EXISTS" || echo "NO_DB"
```

- If `DB_EXISTS` → follow the full protocol below
- If `NO_DB` → skip memory operations gracefully, work without institutional memory

### Pipeline Start (orchestrator responsibility)
Every `/workflow:*` command creates a run entry at the **very beginning**, before invoking any agent:

```bash
# Register the workflow run
sqlite3 .claude/memory.db "INSERT INTO workflow_runs (type, description, scope) VALUES ('WORKFLOW_TYPE', 'USER_DESCRIPTION', 'SCOPE_OR_NULL');"

# Capture the run_id — this is passed to EVERY agent in the chain
RUN_ID=$(sqlite3 .claude/memory.db "SELECT last_insert_rowid();")
```

Replace `WORKFLOW_TYPE` with: `new`, `new-feature`, `improve`, `bugfix`, `audit`, `docs`, `sync`, etc.

### Briefing (MANDATORY — before every agent starts work)
Before doing any work on a scope, the agent queries the DB. Replace `$SCOPE` with the module, domain, or file path being worked on:

```bash
# 1. HOTSPOTS — what files are fragile in my scope?
sqlite3 .claude/memory.db "SELECT file_path, risk_level, times_touched FROM hotspots WHERE file_path LIKE '%$SCOPE%' ORDER BY times_touched DESC LIMIT 5;"

# 2. FAILED APPROACHES — what already failed? DON'T repeat it.
sqlite3 .claude/memory.db "SELECT approach, failure_reason FROM failed_approaches WHERE domain LIKE '%$SCOPE%' ORDER BY id DESC LIMIT 5;"

# 3. OPEN FINDINGS — what's known to be broken?
sqlite3 .claude/memory.db "SELECT finding_id, severity, description FROM findings WHERE file_path LIKE '%$SCOPE%' AND status='open' ORDER BY severity LIMIT 10;"

# 4. ACTIVE DECISIONS — what was decided and why?
sqlite3 .claude/memory.db "SELECT decision, rationale FROM decisions WHERE domain LIKE '%$SCOPE%' AND status='active' ORDER BY id DESC LIMIT 5;"

# 5. KNOWN PATTERNS — what patterns should I follow?
sqlite3 .claude/memory.db "SELECT name, description FROM patterns WHERE domain LIKE '%$SCOPE%';"

# 6. PAST BUGS — what broke before in this area?
sqlite3 .claude/memory.db "SELECT description, root_cause, fix_description FROM bugs WHERE affected_files LIKE '%$SCOPE%' ORDER BY id DESC LIMIT 5;"
```

**How to use the results:**
- If `failed_approaches` returns results → **do NOT retry those approaches**. Start from a different angle.
- If `hotspots` shows `risk_level='high'` or `'critical'` → **be extra careful**, add more tests, review more thoroughly.
- If `findings` shows open P0/P1 → **address them** if they're in your scope.
- If `decisions` exist → **respect them** unless you have a strong reason to supersede (and document why).
- If `patterns` exist → **follow them** for consistency.

### Incremental Logging (MANDATORY — during work)
Log to memory.db **immediately after each significant action**. Do NOT batch entries for the end of your work — if context compaction occurs, batched entries are lost forever. Incremental logging IS the checkpoint mechanism.

**When to log** (within seconds of the action, not later):

| Trigger | What to INSERT |
|---------|---------------|
| After modifying a file | `changes` + `hotspots` upsert |
| After making a design/implementation decision | `decisions` |
| After an approach fails (even partially) | `failed_approaches` |
| After discovering a bug | `bugs` |
| After completing a discrete unit of work | `outcomes` (self-score) |
| After discovering a reusable pattern | `patterns` |
| After discovering a component dependency | `dependencies` |
| After defining a requirement (analyst) | `requirements` |
| After identifying a finding (reviewer/QA) | `findings` |

```bash
# AFTER MODIFYING A FILE — log immediately
sqlite3 .claude/memory.db "INSERT INTO changes (run_id, file_path, change_type, description, agent) VALUES ($RUN_ID, 'path/to/file.rs', 'modified', 'What changed and WHY', 'developer');"
sqlite3 .claude/memory.db "INSERT INTO hotspots (file_path, times_touched, description) VALUES ('path/to/file.rs', 1, 'Why it was touched') ON CONFLICT(file_path) DO UPDATE SET times_touched = times_touched + 1, last_updated = datetime('now');"

# AFTER A DECISION — log immediately
sqlite3 .claude/memory.db "INSERT INTO decisions (run_id, domain, decision, rationale, alternatives, confidence) VALUES ($RUN_ID, 'module-name', 'What was decided', 'Why this choice', '[\"rejected alternative 1: reason\", \"rejected alternative 2: reason\"]', 0.9);"

# AFTER A FAILED APPROACH — log immediately (THE MOST VALUABLE ENTRY)
sqlite3 .claude/memory.db "INSERT INTO failed_approaches (run_id, domain, problem, approach, failure_reason, file_paths) VALUES ($RUN_ID, 'module-name', 'What I was trying to solve', 'What I tried', 'Why it did not work', '[\"file1.rs\", \"file2.rs\"]');"

# AFTER FINDING A BUG — log immediately
sqlite3 .claude/memory.db "INSERT INTO bugs (run_id, description, symptoms, root_cause, fix_description, affected_files) VALUES ($RUN_ID, 'Bug description', 'Error messages or behavior seen', 'Root cause', 'How it was fixed', '[\"files\"]');"

# AFTER COMPLETING A UNIT OF WORK — self-score immediately
sqlite3 .claude/memory.db "INSERT INTO outcomes (run_id, agent, score, domain, action, lesson) VALUES ($RUN_ID, 'AGENT_NAME', SCORE, 'domain', 'What I did', 'What I learned');"

# AFTER IDENTIFYING A FINDING (reviewer/QA) — log immediately
sqlite3 .claude/memory.db "INSERT INTO findings (run_id, finding_id, severity, category, description, file_path, line_range) VALUES ($RUN_ID, 'AUDIT-P1-001', 'P1', 'bug', 'Description', 'file_path', '42-58');"

# AFTER DEFINING A REQUIREMENT (analyst) — log immediately
sqlite3 .claude/memory.db "INSERT OR IGNORE INTO requirements (run_id, req_id, domain, description, priority) VALUES ($RUN_ID, 'REQ-XXX-001', 'domain', 'Requirement description', 'Must');"

# AFTER DISCOVERING A PATTERN — log immediately
sqlite3 .claude/memory.db "INSERT INTO patterns (run_id, domain, name, description, example_files) VALUES ($RUN_ID, 'domain', 'Pattern name', 'When and how to use it', '[\"example_files\"]');"

# AFTER DISCOVERING A DEPENDENCY — log immediately
sqlite3 .claude/memory.db "INSERT OR IGNORE INTO dependencies (source_file, target_file, relationship, discovered_run) VALUES ('caller.rs', 'callee.rs', 'calls', $RUN_ID);"
```

**Rules for incremental logging:**
- Log **immediately** — each INSERT happens within seconds of the action it describes
- Log **every** file change, not just the important ones
- Log **every** failed approach, even small ones — these are the most valuable entries in the entire DB
- Log decisions with the **alternatives you rejected and why** — future sessions need to know what was considered
- If a DB write fails, **log the error and continue working** — never block work for a failed INSERT
- If you don't have a `$RUN_ID`, get it: `sqlite3 .claude/memory.db "SELECT MAX(id) FROM workflow_runs;"`

### Close-Out (MANDATORY — when agent completes or stops)
When your work is complete (or when stopping due to context budget / errors), run a lightweight verification:

1. **Verify completeness** — review what you logged incrementally. Are there any file changes, decisions, or failed approaches you forgot to log? Insert them now.
2. **Final self-scoring** — score any remaining significant actions you haven't yet scored.
3. **Check for lesson distillation** — do 3+ recent outcomes share a theme? If so, distill a lesson (see Self-Learning below).
4. **Reinforce/supersede lessons** — if you confirmed an existing lesson, reinforce it. If one no longer applies, supersede it.

If context is tight, the close-out can be minimal — the important data was already logged incrementally. At minimum, ensure at least one outcome is logged (required for git commits).

### Pipeline End (orchestrator responsibility)
When the workflow completes or fails:

```bash
# On success
sqlite3 .claude/memory.db "UPDATE workflow_runs SET status='completed', completed_at=datetime('now'), git_commits='[\"commit_hash1\", \"commit_hash2\"]' WHERE id=$RUN_ID;"

# On failure
sqlite3 .claude/memory.db "UPDATE workflow_runs SET status='failed', completed_at=datetime('now'), error_message='What failed and why' WHERE id=$RUN_ID;"

# On partial (context budget hit, will resume)
sqlite3 .claude/memory.db "UPDATE workflow_runs SET status='partial', completed_at=datetime('now'), error_message='Context budget reached at step X' WHERE id=$RUN_ID;"
```

### Non-Pipeline Sessions
When working **outside** a formal `/workflow:*` command (e.g., user asks for a quick fix, a one-off question, or manual work):

1. **Still check the DB** — run the briefing queries for the area you're working on
2. **Still write back** — create a workflow_run with type `'manual'` and log what you did
3. The point is that **no work goes unrecorded**, even informal work

```bash
sqlite3 .claude/memory.db "INSERT INTO workflow_runs (type, description) VALUES ('manual', 'Quick fix for X');"
RUN_ID=$(sqlite3 .claude/memory.db "SELECT last_insert_rowid();")
# ... do work, run briefing, log incrementally, close out ...
sqlite3 .claude/memory.db "UPDATE workflow_runs SET status='completed', completed_at=datetime('now') WHERE id=$RUN_ID;"
```

### Self-Learning (Outcome Scoring + Lesson Distillation)

The institutional memory doesn't just record *what happened* — it evaluates *how well it worked* and distills patterns for future sessions. This is a two-tier reward-based learning loop where agents evaluate their own behavior.

#### Tier 1: Outcomes (Working Memory)

**Immediately after** every significant action (module implementation, test creation, design decision, bug fix, approach selection), the agent self-scores — do not batch these for the end:

```bash
sqlite3 .claude/memory.db "INSERT INTO outcomes (run_id, agent, score, domain, action, lesson) VALUES (\$RUN_ID, 'AGENT_NAME', SCORE, 'DOMAIN', 'What I did', 'What I learned from doing it');"
```

**Scoring guide:**
| Score | Meaning | When to use |
|-------|---------|-------------|
| **+1** | Helpful | Approach succeeded cleanly, minimal iteration, good result worth repeating |
| **0** | Neutral | Worked but unremarkable, nothing to learn from this |
| **-1** | Unhelpful | Approach failed, required excessive iteration, hit walls, produced suboptimal result |

**Rules:**
- Score **every significant action**, not just the final result
- Be **honest** — a -1 is more valuable than a false +1
- Include **specific context** in the lesson text, not vague statements
- Bad: `"it worked"` → Good: `"Option<T> pattern avoided unwrap panic in concurrent queue access"`

**During briefing**, the 15 most recent outcomes for the scope are injected:
```bash
sqlite3 .claude/memory.db "SELECT agent, score, action, lesson FROM outcomes WHERE domain LIKE '%\$SCOPE%' ORDER BY id DESC LIMIT 15;"
```

**How to use outcomes in briefing:**
- If recent outcomes show repeated -1 scores for an approach → **avoid that approach**
- If recent outcomes show +1 for a technique → **prefer that technique**
- If the domain's average score is negative → **slow down, be extra careful, consider asking for help**

#### Tier 2: Lessons (Long-Term Memory)

When patterns emerge from repeated outcomes, the agent distills them into **permanent rules**:

```bash
# Content-based dedup: same rule text bumps occurrences instead of duplicating
sqlite3 .claude/memory.db "INSERT INTO lessons (domain, content, source_agent) VALUES ('DOMAIN', 'The distilled rule', 'AGENT_NAME') ON CONFLICT(domain, content) DO UPDATE SET occurrences = occurrences + 1, confidence = MIN(1.0, confidence + 0.1), last_reinforced = datetime('now');"
```

**When to distill a lesson:**
- You notice **3+ outcomes in the same domain** sharing a theme
- You confirm an approach that **consistently works** (+1) or **consistently fails** (-1)
- You discover a **non-obvious pattern** that future agents would benefit from

**Lesson constraints:**
- **Cap of 10 active lessons per domain** — oldest/lowest-confidence pruned during maintenance
- **Content-based dedup** — if the exact lesson already exists, occurrences bumps automatically
- **Confidence grows** with reinforcement (+0.1 per confirmation, max 1.0)
- **Confidence decays** without reinforcement (-0.1 every 30 days unreinforced)

**During briefing**, all active lessons for the scope are injected:
```bash
sqlite3 .claude/memory.db "SELECT content, occurrences, confidence FROM lessons WHERE domain LIKE '%\$SCOPE%' AND status='active' ORDER BY confidence DESC;"
```

**How to use lessons in briefing:**
- Lessons with `confidence >= 0.8` → **treat as established rules**, follow them
- Lessons with `confidence >= 0.5` → **strong guidance**, follow unless you have specific reason not to
- Lessons with `confidence < 0.5` → **emerging patterns**, consider but don't blindly follow
- If a lesson no longer applies → **supersede it**: `UPDATE lessons SET status='superseded' WHERE ...`

#### The Loop

```
Agent starts → briefing injects outcomes + lessons
  → Agent works → logs changes, decisions, failures, outcomes INCREMENTALLY to memory.db
  → Agent close-out: verifies completeness, distills lessons, reinforces/supersedes
  → Next agent (or next session) gets updated context
```

#### Self-Learning During Work (incremental)

Self-scoring happens **immediately after each significant action**, not batched at the end:

```bash
# IMMEDIATELY AFTER each significant action — self-score
sqlite3 .claude/memory.db "INSERT INTO outcomes (run_id, agent, score, domain, action, lesson) VALUES (\$RUN_ID, 'AGENT', 1, 'domain', 'What I did', 'What I learned');"
```

#### Self-Learning at Close-Out (lesson distillation)

During the close-out phase, check for patterns and distill lessons:

```bash
# 1. CHECK FOR DISTILLATION — do recent outcomes suggest a pattern?
sqlite3 .claude/memory.db "SELECT score, action, lesson FROM outcomes WHERE domain='DOMAIN' AND agent='AGENT' ORDER BY id DESC LIMIT 10;"
# If 3+ share a theme → distill:

# 2. DISTILL LESSON (only if pattern detected)
sqlite3 .claude/memory.db "INSERT INTO lessons (domain, content, source_agent) VALUES ('domain', 'The pattern rule', 'agent') ON CONFLICT(domain, content) DO UPDATE SET occurrences = occurrences + 1, confidence = MIN(1.0, confidence + 0.1), last_reinforced = datetime('now');"

# 3. REINFORCE existing lessons you confirmed during work
sqlite3 .claude/memory.db "UPDATE lessons SET occurrences = occurrences + 1, confidence = MIN(1.0, confidence + 0.1), last_reinforced = datetime('now') WHERE domain='domain' AND content LIKE '%pattern%';"

# 4. SUPERSEDE lessons that no longer apply
sqlite3 .claude/memory.db "UPDATE lessons SET status='superseded' WHERE domain='domain' AND content LIKE '%outdated pattern%';"
```

### Error Handling
- If `sqlite3` command fails → **log the error but continue working**. Never block work because the DB is inaccessible.
- If a query returns empty results → that's normal for new projects. Proceed without institutional context.
- If the DB is corrupted → inform the user, suggest re-initializing with `bash scripts/db-init.sh`.

## Main Workflow

```
Raw Idea ("build a CRM tool")
  → [Pipeline start: register workflow_run in memory.db]
  → Discovery (explores, challenges, clarifies the IDEA with the user)
  → Idea Brief (clear, validated concept)
  → Feature Evaluator (GO/NO-GO gate: scores necessity, impact, complexity, alternatives, alignment, risk, timing)
  → Analyst (BA: requirements, acceptance criteria, MoSCoW priorities, traceability)
  → Architect (design with failure modes, security, performance budgets)
  → Test Writer (TDD by priority: Must first, then Should, then Could)
  → Developer (implements module by module)
  → Compiler (automatic validation)
  → QA (end-to-end validation, acceptance criteria verification, exploratory testing)
  → Reviewer (audits code, verifies specs/docs accuracy)
  → Git (automatic versioning)
  → [Pipeline end: close workflow_run, update memory.db]
```

## Traceability Chain
Every requirement flows through the entire pipeline via unique IDs:
```
Discovery validates the idea → Analyst assigns REQ-XXX-001 → Architect maps to module → Test Writer writes TEST-XXX-001 → Developer implements → QA verifies acceptance criteria → Reviewer audits completeness
```
All IDs are also stored in the `requirements` table in memory.db for cross-session tracking.

## Global Rules

1. **NEVER write code without tests first** (strict TDD)
2. **NEVER assume** — if something is unclear, the analyst must ask
3. **Module by module** — do not implement everything at once
4. **Document before coding** — architecture is defined first
5. **Every assumption must be explicit** — technical + human-readable summary
6. **Codebase is king** — when in doubt, read the actual code
7. **Keep specs/ and docs/ in sync** — every code change must update relevant specs and docs
8. **Every requirement has acceptance criteria** — "it should work" is not acceptable
9. **Every requirement has a priority** — Must/Should/Could/Won't (MoSCoW)
10. **Every requirement is traceable** — from ID through tests to implementation
11. **60% context budget** — every agent must complete its work within 60% of the context window
12. **Briefing before action** — every agent queries memory.db before starting work
13. **Log incrementally during work** — every agent writes to memory.db immediately after each significant action, not batched at the end
14. **Self-score every action** — every agent rates its own significant actions (-1/0/+1) immediately after completing them
15. **Distill lessons from patterns** — when 3+ outcomes share a theme, distill into a permanent lesson

## Fail-Safe Controls

The workflow enforces guardrails at every level to prevent silent failures, infinite loops, and cascading garbage.

### Prerequisite Gates
Every agent that receives upstream output verifies its input exists before proceeding. If required input is missing, the agent **STOPS** with a clear error message identifying what's missing and which upstream agent failed.

| Agent | Required Input |
|-------|---------------|
| Analyst (after discovery) | `docs/.workflow/idea-brief.md` |
| Architect | Analyst requirements file in `specs/` |
| Test Writer | Architect design + Analyst requirements in `specs/` |
| Developer | Test files must exist |
| QA | Source code + test files must exist |
| Reviewer | Source code must exist |

### Iteration Limits
Multi-step commands enforce maximum iteration counts to prevent infinite loops:
- **QA ↔ Developer loops:** Maximum **3 iterations**
- **Reviewer ↔ Developer loops:** Maximum **2 iterations**
- **Audit --fix developer attempts:** Maximum **5** per finding (then escalated)
- **Audit --fix build/lint retries:** Maximum **3** per priority pass
- **Audit --fix verification iterations:** Maximum **2** per priority pass

If the limit is reached, the workflow STOPS and reports remaining issues to the user for a human decision.

### Inter-Step Output Validation
Multi-step commands verify that each agent produced its expected output file before invoking the next agent. If output is missing, the chain halts with a clear report of which step failed.

### Error Recovery
If any agent fails mid-chain, the workflow saves chain state to `docs/.workflow/chain-state.md` and updates memory.db with the failure. The user can resume with `/workflow:resume`.

### Directory Safety
Every agent that writes output files verifies target directories exist before writing. If a directory is missing, the agent creates it.

### Developer Max Retry
The developer has a maximum of **5 attempts** per test-fix cycle for a single module. If tests still fail after 5 attempts, the developer stops and escalates for human review or architecture reassessment.

### Language-Agnostic Patterns
Test-writer and reviewer adapt their patterns to the project's language (detected from config files, architect design, or existing source). No agent assumes a specific language.

## Context Window Management

### Critical Rules
- **NEVER read the entire codebase at once** — always scope to the relevant area
- **Read indexes first** — start with `specs/SPECS.md` or `docs/DOCS.md` to identify which files matter
- **Query memory.db first** — check what's already known before reading files
- **Scope narrowing** — all commands accept an optional scope parameter to limit the area of work
- **Chunking** — for large operations (audit, sync, docs), work one milestone/domain at a time

### Agent Scoping Strategy
1. Query memory.db for context on the target area (hotspots, decisions, failures)
2. Read the master index (`specs/SPECS.md`) to understand the project layout
3. Identify which domains/milestones are relevant to the task
4. Read ONLY the relevant spec files and code files
5. If you feel context getting heavy, stop and summarize what you've learned so far before continuing

### Scope Parameter
All workflow commands accept an optional scope to limit context usage:
```
/workflow:new-feature "add retry logic" --scope="providers"
/workflow:audit --scope="milestone 3: core"
/workflow:sync --scope="memory"
/workflow:bugfix "scheduler crash" --scope="backend/src/gateway/scheduler.rs"
```

When no scope is provided, the analyst determines the minimal scope needed based on the task description.

### 60% Context Budget
Every agent operates under a **60% context window budget**. This is a proactive limit, not a reactive fallback — agents plan their work to fit within 60%, leaving 40% headroom for reasoning, edge cases, and unexpected complexity.

**Why 60%:** Agents that consume their full context window produce degraded output — they lose track of earlier decisions, repeat themselves, and miss connections. The 60% budget ensures each agent finishes strong with full recall of its work.

**How it works:**
- The **Architect** sizes milestones so each downstream agent can complete one milestone within 60% of its context (max 3 modules per milestone)
- Each **pipeline agent** monitors its own usage proactively and stops at the 60% mark
- When an agent hits the budget, it saves state to `docs/.workflow/` and the pipeline continues via `/workflow:resume`

**Heuristics for agents:**
- If you've read more than ~20 files without saving progress, you are likely near the budget
- If you've processed more than 3 modules without checkpointing, save progress now
- If you're on your second major domain/area, consider whether you have enough budget remaining for the rest

### When Reaching the 60% Budget
When an agent reaches 60% of its context window:
1. **Summarize** what has been learned so far into a temporary file at `docs/.workflow/[agent]-[task]-summary.md`
2. **Delegate** remaining work by spawning a continuation subagent that reads the summary
3. **Never silently degrade** — if you can't do a thorough job, say so and suggest splitting the task

## Project Layout

```
root-project/
├── backend/              ← Backend source code (Rust or preferred language)
├── frontend/             ← Frontend source code (if applicable)
├── specs/                ← Technical specifications (at project root)
├── docs/                 ← Documentation (at project root)
├── CLAUDE.md             ← Workflow rules
└── .claude/
    ├── agents/           ← Agent definitions (deployed by setup.sh)
    ├── commands/         ← Command definitions (deployed by setup.sh)
    ├── memory.db         ← Institutional memory (SQLite)
    └── db-queries/       ← Query reference files for agents
```

Code lives in `backend/` (and optionally `frontend/`). Specs and docs remain at the project root.
Agents must be aware of this structure when scoping reads and writes.

## Documentation Structure

### specs/ (technical specifications)
```
specs/
├── SPECS.md              ← Master index (links to all domain specs)
├── core-config.md        ← Per-domain spec files
├── core-context.md
├── memory-store.md
├── channels-telegram.md
└── ...
```
- One spec file per domain/module/file
- SPECS.md must be updated when new specs are added
- Specs describe WHAT the code does technically

### docs/ (documentation)
```
docs/
├── DOCS.md               ← Master index (links to all doc files)
├── quickstart.md
├── architecture.md
├── configuration.md
└── ...
```
- Topic-oriented guides and references
- DOCS.md must be updated when new docs are added
- Docs describe HOW to use/understand the system

## Usage Modes

### New project from scratch
```
/workflow:new "description of the idea"
```
Full chain: discovery → analyst → architect → test-writer → developer → QA → reviewer.

### Add feature to existing project
```
/workflow:new-feature "description of the feature" [--scope="area"]
```
Full chain: (discovery if vague) → feature-evaluator (GO/NO-GO) → analyst → architect → test-writer → developer → QA → reviewer.

### Improve existing code
```
/workflow:improve "description of the improvement" [--scope="area"]
```
Reduced chain (no architect): analyst → test-writer (regression) → developer (refactor) → QA → reviewer

### Fix a bug
```
/workflow:bugfix "description of the bug" [--scope="file or module"]
```
Reduced chain: analyst → test-writer (reproduces the bug) → developer → QA → reviewer

### Audit existing code
```
/workflow:audit [--scope="milestone or module"]
/workflow:audit --fix [--scope="area"] [--include-p3]
```

### Document existing project
```
/workflow:docs [--scope="milestone or module"]
```

### Sync specs and docs with codebase
```
/workflow:sync [--scope="milestone or module"]
```

### Map codebase functionalities
```
/workflow:functionalities [--scope="module or area"]
```

### Understand a codebase
```
/workflow:understand [--scope="module or area"]
```

### Create a new agent role
```
/workflow:create-role "description of the desired role"
```

### Audit an agent role definition
```
/workflow:audit-role ".claude/agents/[name].md" [--scope="dimensions"]
```

### Resume a stopped workflow
```
/workflow:resume [--from="M3" or --from="developer"]
```

### Design a wizard or setup flow
```
/workflow:wizard-ux "description of wizard" [--scope="medium"]
```

### Diagnose a hard bug
```
/workflow:diagnose "description of the bug" [--scope="subsystem"]
/workflow:diagnose "description of the bug" --fix [--scope="subsystem"]
```

## Conventions
- Preferred language: Rust (or whatever the user defines)
- Tests: alongside code or in `backend/tests/` (or `frontend/tests/`) folder
- Commits: conventional (feat:, fix:, docs:, refactor:, test:)
- Branches: feature/, bugfix/, hotfix/
