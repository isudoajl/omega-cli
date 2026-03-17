#!/bin/bash
# ============================================================
# SESSION CLEANUP — Notification hook
# Cleans up session flag files and promotes hotspot risk levels.
# Runs on notifications (closest event to session lifecycle).
# ============================================================

# Consume stdin (hook receives JSON input)
cat > /dev/null

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}"
DB_PATH="$PROJECT_DIR/.claude/memory.db"

# Graceful exit if no DB
if [ ! -f "$DB_PATH" ]; then
    exit 0
fi

# Promote hotspot risk levels based on touch counts
sqlite3 "$DB_PATH" "UPDATE hotspots SET risk_level = 'critical' WHERE times_touched >= 10 AND risk_level != 'critical';" 2>/dev/null || true
sqlite3 "$DB_PATH" "UPDATE hotspots SET risk_level = 'high' WHERE times_touched >= 5 AND times_touched < 10 AND risk_level NOT IN ('critical', 'high');" 2>/dev/null || true
sqlite3 "$DB_PATH" "UPDATE hotspots SET risk_level = 'medium' WHERE times_touched >= 3 AND times_touched < 5 AND risk_level NOT IN ('critical', 'high', 'medium');" 2>/dev/null || true

exit 0
