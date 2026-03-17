#!/bin/bash
# ============================================================
# INCREMENTAL GATE — PreToolUse hook (matcher: Write, Edit)
# Blocks file modifications after a threshold (10) without any
# outcomes logged to memory.db. This enforces incremental
# logging even when the agent never reaches a git commit.
# ============================================================

# Read hook input from stdin
INPUT=$(cat)

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}"
DB_PATH="$PROJECT_DIR/.claude/memory.db"
BRIEFING_FLAG="$PROJECT_DIR/.claude/hooks/.briefing_done"
EDIT_COUNTER="$PROJECT_DIR/.claude/hooks/.edit_counter"

# If no DB, allow all edits
if [ ! -f "$DB_PATH" ]; then
    exit 0
fi

# Get session start time
SESSION_START=""
if [ -f "$BRIEFING_FLAG" ]; then
    STORED_DATA=$(cat "$BRIEFING_FLAG" 2>/dev/null || echo "")
    SESSION_START=$(echo "$STORED_DATA" | cut -d'|' -f2)
fi

if [ -z "$SESSION_START" ]; then
    SESSION_START=$(sqlite3 "$DB_PATH" "SELECT datetime('now', '-30 minutes');" 2>/dev/null || echo "")
fi

# Check if any outcomes were logged since session start
OUTCOME_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM outcomes WHERE created_at >= '$SESSION_START';" 2>/dev/null || echo "0")

if [ "$OUTCOME_COUNT" -gt 0 ]; then
    # Has outcomes — reset counter and allow
    rm -f "$EDIT_COUNTER" 2>/dev/null
    exit 0
fi

# Increment edit counter
mkdir -p "$(dirname "$EDIT_COUNTER")"
if [ -f "$EDIT_COUNTER" ]; then
    COUNT=$(cat "$EDIT_COUNTER" 2>/dev/null || echo "0")
    COUNT=$((COUNT + 1))
else
    COUNT=1
fi
echo "$COUNT" > "$EDIT_COUNTER"

# Block after 10 file modifications without outcomes
if [ "$COUNT" -ge 10 ]; then
    echo "EDIT BLOCKED — INCREMENTAL LOGGING REQUIRED"
    echo ""
    echo "$COUNT file modifications this session without logging to memory.db."
    echo "Before continuing, you MUST log at least one outcome:"
    echo ""
    echo "  1. Register this session (if not done):"
    echo "     sqlite3 .claude/memory.db \"INSERT INTO workflow_runs (type, description) VALUES ('manual', 'description');\""
    echo "     RUN_ID=\$(sqlite3 .claude/memory.db \"SELECT last_insert_rowid();\")"
    echo ""
    echo "  2. Self-score your work so far:"
    echo "     sqlite3 .claude/memory.db \"INSERT INTO outcomes (run_id, agent, score, domain, action, lesson) VALUES (\$RUN_ID, 'developer', 1, 'domain', 'what you did', 'what you learned');\""
    echo ""
    echo "  3. Log file changes:"
    echo "     sqlite3 .claude/memory.db \"INSERT INTO changes (run_id, file_path, change_type, description, agent) VALUES (\$RUN_ID, 'path', 'modified', 'description', 'developer');\""
    echo ""
    echo "Then continue editing."
    exit 2
fi

exit 0
