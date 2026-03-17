#!/bin/bash
# ============================================================
# DEBRIEF NUDGE — PostToolUse hook
# After tool executions, checks if debrief is overdue for
# THIS SESSION. Throttled: only reminds every 5th tool call.
# ============================================================

# Consume stdin (hook receives JSON input)
cat > /dev/null

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}"
DB_PATH="$PROJECT_DIR/.claude/memory.db"
BRIEFING_FLAG="$PROJECT_DIR/.claude/hooks/.briefing_done"
COUNTER_FILE="$PROJECT_DIR/.claude/hooks/.nudge_counter"

# If no DB, stay silent
if [ ! -f "$DB_PATH" ]; then
    exit 0
fi

# Get the briefing timestamp for this session
SESSION_START=""
if [ -f "$BRIEFING_FLAG" ]; then
    STORED_DATA=$(cat "$BRIEFING_FLAG" 2>/dev/null || echo "")
    SESSION_START=$(echo "$STORED_DATA" | cut -d'|' -f2)
fi

if [ -z "$SESSION_START" ]; then
    SESSION_START=$(sqlite3 "$DB_PATH" "SELECT datetime('now', '-30 minutes');" 2>/dev/null || echo "")
fi

# Check if outcomes were logged since this session's briefing
OUTCOME_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM outcomes WHERE created_at >= '$SESSION_START';" 2>/dev/null || echo "0")

if [ "$OUTCOME_COUNT" -gt 0 ]; then
    rm -f "$COUNTER_FILE" 2>/dev/null
    exit 0
fi

# Increment tool call counter
mkdir -p "$(dirname "$COUNTER_FILE")"
if [ -f "$COUNTER_FILE" ]; then
    COUNT=$(cat "$COUNTER_FILE" 2>/dev/null || echo "0")
    COUNT=$((COUNT + 1))
else
    COUNT=1
fi
echo "$COUNT" > "$COUNTER_FILE"

# Only nudge every 5th tool call
if [ $((COUNT % 5)) -ne 0 ]; then
    exit 0
fi

echo ""
echo "[INCREMENTAL LOGGING REMINDER: $COUNT tool calls without logging to memory.db. Log changes, decisions, failed approaches, and outcomes as you work — not just at the end. Git commits will be blocked until at least one outcome is logged.]"

exit 0
