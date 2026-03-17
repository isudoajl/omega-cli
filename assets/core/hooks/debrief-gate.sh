#!/bin/bash
# ============================================================
# DEBRIEF GATE — PreToolUse hook (matcher: Bash)
# Blocks git commits unless the AI has logged at least one
# outcome (self-score) in THIS SESSION. Uses the briefing
# timestamp from .briefing_done to scope the check.
# ============================================================

# Read hook input from stdin
INPUT=$(cat)

# Extract the command from the JSON input
COMMAND=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_input',{}).get('command',''))" 2>/dev/null || echo "")

# Only gate git commits — let everything else through immediately
case "$COMMAND" in
    *"git commit"*)
        ;;
    *)
        exit 0
        ;;
esac

# --- This is a git commit. Check if debrief was done THIS SESSION. ---

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}"
DB_PATH="$PROJECT_DIR/.claude/memory.db"
BRIEFING_FLAG="$PROJECT_DIR/.claude/hooks/.briefing_done"

# If no DB, allow the commit
if [ ! -f "$DB_PATH" ]; then
    exit 0
fi

# Get the briefing timestamp for this session (set by briefing.sh)
SESSION_START=""
if [ -f "$BRIEFING_FLAG" ]; then
    STORED_DATA=$(cat "$BRIEFING_FLAG" 2>/dev/null || echo "")
    SESSION_START=$(echo "$STORED_DATA" | cut -d'|' -f2)
fi

# If no briefing timestamp, fall back to 30-minute window
if [ -z "$SESSION_START" ]; then
    SESSION_START=$(sqlite3 "$DB_PATH" "SELECT datetime('now', '-30 minutes');" 2>/dev/null || echo "")
fi

# Check if any outcomes were logged AFTER this session's briefing
OUTCOME_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM outcomes WHERE created_at >= '$SESSION_START';" 2>/dev/null || echo "0")

if [ "$OUTCOME_COUNT" -gt 0 ]; then
    # Debrief happened this session — allow the commit
    exit 0
fi

# --- BLOCKED ---
echo "COMMIT BLOCKED — DEBRIEF REQUIRED FOR THIS SESSION"
echo ""
echo "No outcomes (self-scores) logged since this session started ($SESSION_START)."
echo "Before committing, you MUST:"
echo ""
echo "  1. Register this session:"
echo "     sqlite3 .claude/memory.db \"INSERT INTO workflow_runs (type, description) VALUES ('manual', 'description');\""
echo "     RUN_ID=\$(sqlite3 .claude/memory.db \"SELECT last_insert_rowid();\")"
echo ""
echo "  2. Self-score your significant actions:"
echo "     sqlite3 .claude/memory.db \"INSERT INTO outcomes (run_id, agent, score, domain, action, lesson) VALUES (\\\$RUN_ID, 'developer', 1, 'domain', 'what you did', 'what you learned');\""
echo ""
echo "  3. Then retry the commit."
exit 2
