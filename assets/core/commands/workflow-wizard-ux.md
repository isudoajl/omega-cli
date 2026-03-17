---
name: workflow:wizard-ux
description: Design an intuitive installation wizard, setup flow, or onboarding sequence for TUI, GUI, Web, or CLI. Produces a complete wizard flow specification with step definitions, validation rules, UX copy, error recovery, and accessibility requirements.
---

# Workflow: Wizard UX Design

The user wants to design a wizard flow -- an installation wizard, setup assistant, onboarding sequence, or multi-step configuration process.

Input: a description of what the wizard should accomplish, plus optionally the target medium (TUI/GUI/Web/CLI) and a `--scope` parameter to limit the design area.

## Single Agent Workflow

This workflow invokes the `wizard-ux` subagent, which handles the full design lifecycle:

1. **Understand** what the wizard should configure and who the users are
2. **Analyze** the target medium's capabilities and constraints
3. **Design** each step with fields, defaults, validation, UX copy, and error handling
4. **Architect** the flow (step sequence, conditional branches, state management, navigation rules)
5. **Audit** accessibility and error recovery
6. **Present** the complete specification for user approval
7. **Save** the approved specification to `specs/[domain]-wizard-flow.md`

## Invocation

Invoke the `wizard-ux` subagent with the user's wizard description and any `--scope` or medium specification.

Examples:
```
/workflow:wizard-ux "design a setup wizard for first-time database configuration" --scope="TUI"
/workflow:wizard-ux "create an onboarding flow for new API users" --scope="Web"
/workflow:wizard-ux "installation wizard for our CLI tool"
```

If no medium is specified, the agent will ask. If the wizard scope is too broad, the agent will recommend splitting into multiple focused wizards.

## Fail-Safe Controls

### Human Approval Gate
The agent MUST present the complete wizard flow specification and receive explicit user approval before saving any files. This is non-negotiable.

### Progress Recovery
If the conversation is interrupted or context limits are reached, the agent saves progress to `docs/.workflow/wizard-ux-progress.md`. The user can re-invoke this command -- the agent will read the progress file and continue from where it left off.

### Clarification Limits
- **Clarification rounds:** Maximum 2. If the wizard scope is still unclear after 2 rounds, the agent proceeds with what it knows and flags uncertainties.
- **Design revisions:** No hard limit -- the user can iterate on the specification as many times as needed before approval.

## What It Produces

- `specs/[domain]-wizard-flow.md` -- complete wizard flow specification with:
  - Step definitions (ID, title, purpose, fields, defaults, validation, UX copy)
  - Flow architecture (step sequence, conditional branches, progress model)
  - State management (storage, persistence, recovery)
  - Navigation rules (forward, back, skip, cancel)
  - Error recovery (validation, network, permissions, interruption)
  - Accessibility requirements (keyboard, screen reader, color independence)
  - Expert/fast-path mode (config file, flags, env vars, unattended mode)
  - Medium-specific adaptations (TUI, GUI, Web, CLI variations)
  - Post-wizard experience (success/failure screens, next steps, generated artifacts)

## Integration with Full Pipeline

When used within a larger workflow:
- **After Analyst**: The analyst identifies wizard requirements (e.g., "REQ-SETUP-001: System shall provide guided first-run setup"). The wizard-ux agent designs the UX flow for those requirements
- **Before Architect**: The architect reads the wizard specification to design the technical implementation (framework choice, state management, component architecture)
- **Before Test Writer**: The test writer uses step definitions and validation rules to write wizard flow tests (navigation, validation, error recovery, edge cases)
- **Before Developer**: The developer implements the wizard following the exact specification
