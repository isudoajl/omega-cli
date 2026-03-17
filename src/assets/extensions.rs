//! Extension pack asset definitions.
//!
//! Each extension contributes additional agents and commands that are
//! deployed alongside the core assets when the user opts in.

use super::{Asset, Extension};

// ---------------------------------------------------------------------------
// Blockchain extension (3 agents, 3 commands)
// ---------------------------------------------------------------------------

const BLOCKCHAIN_AGENT_DEBUG: Asset = Asset {
    name: "blockchain-debug.md",
    content: include_str!("../../assets/extensions/blockchain/agents/blockchain-debug.md"),
};
const BLOCKCHAIN_AGENT_NETWORK: Asset = Asset {
    name: "blockchain-network.md",
    content: include_str!("../../assets/extensions/blockchain/agents/blockchain-network.md"),
};
const BLOCKCHAIN_AGENT_STRESS_TESTER: Asset = Asset {
    name: "stress-tester.md",
    content: include_str!("../../assets/extensions/blockchain/agents/stress-tester.md"),
};

static BLOCKCHAIN_AGENTS: [Asset; 3] = [
    BLOCKCHAIN_AGENT_DEBUG,
    BLOCKCHAIN_AGENT_NETWORK,
    BLOCKCHAIN_AGENT_STRESS_TESTER,
];

const BLOCKCHAIN_CMD_DEBUG: Asset = Asset {
    name: "workflow-blockchain-debug.md",
    content: include_str!(
        "../../assets/extensions/blockchain/commands/workflow-blockchain-debug.md"
    ),
};
const BLOCKCHAIN_CMD_NETWORK: Asset = Asset {
    name: "workflow-blockchain-network.md",
    content: include_str!(
        "../../assets/extensions/blockchain/commands/workflow-blockchain-network.md"
    ),
};
const BLOCKCHAIN_CMD_STRESS_TEST: Asset = Asset {
    name: "workflow-stress-test.md",
    content: include_str!("../../assets/extensions/blockchain/commands/workflow-stress-test.md"),
};

static BLOCKCHAIN_COMMANDS: [Asset; 3] = [
    BLOCKCHAIN_CMD_DEBUG,
    BLOCKCHAIN_CMD_NETWORK,
    BLOCKCHAIN_CMD_STRESS_TEST,
];

static BLOCKCHAIN_EXT: Extension = Extension {
    name: "blockchain",
    agents: &BLOCKCHAIN_AGENTS,
    commands: &BLOCKCHAIN_COMMANDS,
};

// ---------------------------------------------------------------------------
// C2C-protocol extension (2 agents, 3 commands)
// ---------------------------------------------------------------------------

const C2C_AGENT_ARCHITECT: Asset = Asset {
    name: "proto-architect.md",
    content: include_str!("../../assets/extensions/c2c-protocol/agents/proto-architect.md"),
};
const C2C_AGENT_AUDITOR: Asset = Asset {
    name: "proto-auditor.md",
    content: include_str!("../../assets/extensions/c2c-protocol/agents/proto-auditor.md"),
};

static C2C_AGENTS: [Asset; 2] = [C2C_AGENT_ARCHITECT, C2C_AGENT_AUDITOR];

const C2C_CMD_C2C: Asset = Asset {
    name: "workflow-c2c.md",
    content: include_str!("../../assets/extensions/c2c-protocol/commands/workflow-c2c.md"),
};
const C2C_CMD_PROTO_AUDIT: Asset = Asset {
    name: "workflow-proto-audit.md",
    content: include_str!("../../assets/extensions/c2c-protocol/commands/workflow-proto-audit.md"),
};
const C2C_CMD_PROTO_IMPROVE: Asset = Asset {
    name: "workflow-proto-improve.md",
    content: include_str!(
        "../../assets/extensions/c2c-protocol/commands/workflow-proto-improve.md"
    ),
};

static C2C_COMMANDS: [Asset; 3] = [C2C_CMD_C2C, C2C_CMD_PROTO_AUDIT, C2C_CMD_PROTO_IMPROVE];

static C2C_EXT: Extension = Extension {
    name: "c2c-protocol",
    agents: &C2C_AGENTS,
    commands: &C2C_COMMANDS,
};

// ---------------------------------------------------------------------------
// All extensions
// ---------------------------------------------------------------------------

static ALL_EXTENSIONS: [Extension; 2] = [BLOCKCHAIN_EXT, C2C_EXT];

/// Returns all available extension packs.
pub fn all_extensions() -> &'static [Extension] {
    &ALL_EXTENSIONS
}
