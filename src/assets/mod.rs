//! Embedded asset registry for the OMEGA CLI.
//!
//! All deployable files are embedded at compile time via `include_str!()`.
//! This module provides typed access to core agents, commands, hooks,
//! SQL schema/queries, workflow rules, scaffold templates, and extensions.

pub mod extensions;

/// A single embedded asset (agent, command, hook, query, etc.).
#[derive(Clone, Copy)]
pub struct Asset {
    pub name: &'static str,
    pub content: &'static str,
}

/// An extension pack containing additional agents and commands.
#[derive(Clone, Copy)]
pub struct Extension {
    pub name: &'static str,
    pub agents: &'static [Asset],
    pub commands: &'static [Asset],
}

// ---------------------------------------------------------------------------
// Core agents (14)
// ---------------------------------------------------------------------------

const AGENT_ANALYST: Asset = Asset {
    name: "analyst.md",
    content: include_str!("../../assets/core/agents/analyst.md"),
};
const AGENT_ARCHITECT: Asset = Asset {
    name: "architect.md",
    content: include_str!("../../assets/core/agents/architect.md"),
};
const AGENT_CODEBASE_EXPERT: Asset = Asset {
    name: "codebase-expert.md",
    content: include_str!("../../assets/core/agents/codebase-expert.md"),
};
const AGENT_DEVELOPER: Asset = Asset {
    name: "developer.md",
    content: include_str!("../../assets/core/agents/developer.md"),
};
const AGENT_DIAGNOSTICIAN: Asset = Asset {
    name: "diagnostician.md",
    content: include_str!("../../assets/core/agents/diagnostician.md"),
};
const AGENT_DISCOVERY: Asset = Asset {
    name: "discovery.md",
    content: include_str!("../../assets/core/agents/discovery.md"),
};
const AGENT_FEATURE_EVALUATOR: Asset = Asset {
    name: "feature-evaluator.md",
    content: include_str!("../../assets/core/agents/feature-evaluator.md"),
};
const AGENT_FUNCTIONALITY_ANALYST: Asset = Asset {
    name: "functionality-analyst.md",
    content: include_str!("../../assets/core/agents/functionality-analyst.md"),
};
const AGENT_QA: Asset = Asset {
    name: "qa.md",
    content: include_str!("../../assets/core/agents/qa.md"),
};
const AGENT_REVIEWER: Asset = Asset {
    name: "reviewer.md",
    content: include_str!("../../assets/core/agents/reviewer.md"),
};
const AGENT_ROLE_AUDITOR: Asset = Asset {
    name: "role-auditor.md",
    content: include_str!("../../assets/core/agents/role-auditor.md"),
};
const AGENT_ROLE_CREATOR: Asset = Asset {
    name: "role-creator.md",
    content: include_str!("../../assets/core/agents/role-creator.md"),
};
const AGENT_TEST_WRITER: Asset = Asset {
    name: "test-writer.md",
    content: include_str!("../../assets/core/agents/test-writer.md"),
};
const AGENT_WIZARD_UX: Asset = Asset {
    name: "wizard-ux.md",
    content: include_str!("../../assets/core/agents/wizard-ux.md"),
};

static CORE_AGENTS: [Asset; 14] = [
    AGENT_ANALYST,
    AGENT_ARCHITECT,
    AGENT_CODEBASE_EXPERT,
    AGENT_DEVELOPER,
    AGENT_DIAGNOSTICIAN,
    AGENT_DISCOVERY,
    AGENT_FEATURE_EVALUATOR,
    AGENT_FUNCTIONALITY_ANALYST,
    AGENT_QA,
    AGENT_REVIEWER,
    AGENT_ROLE_AUDITOR,
    AGENT_ROLE_CREATOR,
    AGENT_TEST_WRITER,
    AGENT_WIZARD_UX,
];

// ---------------------------------------------------------------------------
// Core commands (14)
// ---------------------------------------------------------------------------

const CMD_WORKFLOW_AUDIT: Asset = Asset {
    name: "workflow-audit.md",
    content: include_str!("../../assets/core/commands/workflow-audit.md"),
};
const CMD_WORKFLOW_AUDIT_ROLE: Asset = Asset {
    name: "workflow-audit-role.md",
    content: include_str!("../../assets/core/commands/workflow-audit-role.md"),
};
const CMD_WORKFLOW_BUGFIX: Asset = Asset {
    name: "workflow-bugfix.md",
    content: include_str!("../../assets/core/commands/workflow-bugfix.md"),
};
const CMD_WORKFLOW_CREATE_ROLE: Asset = Asset {
    name: "workflow-create-role.md",
    content: include_str!("../../assets/core/commands/workflow-create-role.md"),
};
const CMD_WORKFLOW_DIAGNOSE: Asset = Asset {
    name: "workflow-diagnose.md",
    content: include_str!("../../assets/core/commands/workflow-diagnose.md"),
};
const CMD_WORKFLOW_DOCS: Asset = Asset {
    name: "workflow-docs.md",
    content: include_str!("../../assets/core/commands/workflow-docs.md"),
};
const CMD_WORKFLOW_FUNCTIONALITIES: Asset = Asset {
    name: "workflow-functionalities.md",
    content: include_str!("../../assets/core/commands/workflow-functionalities.md"),
};
const CMD_WORKFLOW_IMPROVE: Asset = Asset {
    name: "workflow-improve.md",
    content: include_str!("../../assets/core/commands/workflow-improve.md"),
};
const CMD_WORKFLOW_NEW: Asset = Asset {
    name: "workflow-new.md",
    content: include_str!("../../assets/core/commands/workflow-new.md"),
};
const CMD_WORKFLOW_NEW_FEATURE: Asset = Asset {
    name: "workflow-new-feature.md",
    content: include_str!("../../assets/core/commands/workflow-new-feature.md"),
};
const CMD_WORKFLOW_RESUME: Asset = Asset {
    name: "workflow-resume.md",
    content: include_str!("../../assets/core/commands/workflow-resume.md"),
};
const CMD_WORKFLOW_SYNC: Asset = Asset {
    name: "workflow-sync.md",
    content: include_str!("../../assets/core/commands/workflow-sync.md"),
};
const CMD_WORKFLOW_UNDERSTAND: Asset = Asset {
    name: "workflow-understand.md",
    content: include_str!("../../assets/core/commands/workflow-understand.md"),
};
const CMD_WORKFLOW_WIZARD_UX: Asset = Asset {
    name: "workflow-wizard-ux.md",
    content: include_str!("../../assets/core/commands/workflow-wizard-ux.md"),
};

static CORE_COMMANDS: [Asset; 14] = [
    CMD_WORKFLOW_AUDIT,
    CMD_WORKFLOW_AUDIT_ROLE,
    CMD_WORKFLOW_BUGFIX,
    CMD_WORKFLOW_CREATE_ROLE,
    CMD_WORKFLOW_DIAGNOSE,
    CMD_WORKFLOW_DOCS,
    CMD_WORKFLOW_FUNCTIONALITIES,
    CMD_WORKFLOW_IMPROVE,
    CMD_WORKFLOW_NEW,
    CMD_WORKFLOW_NEW_FEATURE,
    CMD_WORKFLOW_RESUME,
    CMD_WORKFLOW_SYNC,
    CMD_WORKFLOW_UNDERSTAND,
    CMD_WORKFLOW_WIZARD_UX,
];

// ---------------------------------------------------------------------------
// Core hooks (5)
// ---------------------------------------------------------------------------

const HOOK_BRIEFING: Asset = Asset {
    name: "briefing.sh",
    content: include_str!("../../assets/core/hooks/briefing.sh"),
};
const HOOK_DEBRIEF_GATE: Asset = Asset {
    name: "debrief-gate.sh",
    content: include_str!("../../assets/core/hooks/debrief-gate.sh"),
};
const HOOK_DEBRIEF_NUDGE: Asset = Asset {
    name: "debrief-nudge.sh",
    content: include_str!("../../assets/core/hooks/debrief-nudge.sh"),
};
const HOOK_INCREMENTAL_GATE: Asset = Asset {
    name: "incremental-gate.sh",
    content: include_str!("../../assets/core/hooks/incremental-gate.sh"),
};
const HOOK_SESSION_CLOSE: Asset = Asset {
    name: "session-close.sh",
    content: include_str!("../../assets/core/hooks/session-close.sh"),
};

static CORE_HOOKS: [Asset; 5] = [
    HOOK_BRIEFING,
    HOOK_DEBRIEF_GATE,
    HOOK_DEBRIEF_NUDGE,
    HOOK_INCREMENTAL_GATE,
    HOOK_SESSION_CLOSE,
];

// ---------------------------------------------------------------------------
// DB schema and queries
// ---------------------------------------------------------------------------

const SCHEMA_SQL_CONTENT: &str = include_str!("../../assets/core/db/schema.sql");

const QUERY_BRIEFING: Asset = Asset {
    name: "briefing.sql",
    content: include_str!("../../assets/core/db/queries/briefing.sql"),
};
const QUERY_DEBRIEF: Asset = Asset {
    name: "debrief.sql",
    content: include_str!("../../assets/core/db/queries/debrief.sql"),
};
const QUERY_MAINTENANCE: Asset = Asset {
    name: "maintenance.sql",
    content: include_str!("../../assets/core/db/queries/maintenance.sql"),
};

static QUERY_FILES: [Asset; 3] = [QUERY_BRIEFING, QUERY_DEBRIEF, QUERY_MAINTENANCE];

// ---------------------------------------------------------------------------
// Workflow rules
// ---------------------------------------------------------------------------

const WORKFLOW_RULES_CONTENT: &str = include_str!("../../assets/core/WORKFLOW_RULES.md");

// ---------------------------------------------------------------------------
// Scaffold templates
// ---------------------------------------------------------------------------

const SCAFFOLD_SPECS_MD: &str = "\
# Specifications Index

> Master index of all technical specifications for this project.

| Domain | Spec File | Description |
|--------|-----------|-------------|
| *(none yet)* | | |

---

*Generated by OMEGA. Update this file when adding new specs.*
";

const SCAFFOLD_DOCS_MD: &str = "\
# Documentation Index

> Master index of all documentation for this project.

| Topic | Doc File | Description |
|-------|----------|-------------|
| *(none yet)* | | |

---

*Generated by OMEGA. Update this file when adding new docs.*
";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Returns all 14 core agent assets.
pub fn core_agents() -> &'static [Asset] {
    &CORE_AGENTS
}

/// Returns all 14 core command assets.
pub fn core_commands() -> &'static [Asset] {
    &CORE_COMMANDS
}

/// Returns all 5 core hook assets.
pub fn core_hooks() -> &'static [Asset] {
    &CORE_HOOKS
}

/// Returns the SQLite schema SQL content.
pub fn schema_sql() -> &'static str {
    SCHEMA_SQL_CONTENT
}

/// Returns all 3 SQL query file assets.
pub fn query_files() -> &'static [Asset] {
    &QUERY_FILES
}

/// Returns the workflow rules markdown content.
pub fn workflow_rules() -> &'static str {
    WORKFLOW_RULES_CONTENT
}

/// Returns the default SPECS.md scaffold template.
pub fn scaffold_specs_md() -> &'static str {
    SCAFFOLD_SPECS_MD
}

/// Returns the default DOCS.md scaffold template.
pub fn scaffold_docs_md() -> &'static str {
    SCAFFOLD_DOCS_MD
}

/// Returns all available extensions.
pub fn extensions() -> &'static [Extension] {
    extensions::all_extensions()
}

/// Looks up an extension by name (case-sensitive).
pub fn extension_by_name(name: &str) -> Option<&'static Extension> {
    extensions().iter().find(|ext| ext.name == name)
}
