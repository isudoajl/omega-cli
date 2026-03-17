-- ============================================================
-- BRIEFING QUERIES — agents run these BEFORE starting work
-- ============================================================
-- Usage: sqlite3 .claude/memory.db < core/db/queries/briefing.sql
-- Or agents call individual queries via sqlite3 CLI

-- 1. File briefing — what do we know about files I'm about to touch?
-- Replace $FILE_PATH with the actual path
-- sqlite3 .claude/memory.db "SELECT * FROM v_file_briefing WHERE file_path LIKE '%scheduler%';"

-- 2. Failed approaches in a domain — DON'T repeat what already failed
-- sqlite3 .claude/memory.db "SELECT approach, failure_reason FROM failed_approaches WHERE domain LIKE '%scheduler%' ORDER BY id DESC LIMIT 5;"

-- 3. Open findings in scope — what's already known to be broken?
-- sqlite3 .claude/memory.db "SELECT finding_id, severity, description, file_path FROM findings WHERE file_path LIKE '%src/scheduler%' AND status='open' ORDER BY severity;"

-- 4. Recent decisions in domain — what was already decided and why?
-- sqlite3 .claude/memory.db "SELECT decision, rationale, alternatives FROM decisions WHERE domain LIKE '%scheduler%' AND status='active' ORDER BY id DESC LIMIT 5;"

-- 5. Hotspot check — is this file known to be fragile?
-- sqlite3 .claude/memory.db "SELECT file_path, risk_level, times_touched, description FROM hotspots WHERE file_path LIKE '%scheduler%';"

-- 6. Related bugs — has this area had bugs before?
-- sqlite3 .claude/memory.db "SELECT description, root_cause, fix_description FROM bugs WHERE affected_files LIKE '%scheduler%' ORDER BY id DESC LIMIT 5;"

-- 7. Requirement status — what's the current state of requirements in this domain?
-- sqlite3 .claude/memory.db "SELECT req_id, description, priority, status FROM requirements WHERE domain LIKE '%scheduler%';"

-- 8. Known patterns in this area
-- sqlite3 .claude/memory.db "SELECT name, description FROM patterns WHERE domain LIKE '%scheduler%';"

-- 9. Recent activity — what workflows ran recently in this area?
-- sqlite3 .claude/memory.db "SELECT type, description, status, started_at FROM workflow_runs WHERE scope LIKE '%scheduler%' OR description LIKE '%scheduler%' ORDER BY id DESC LIMIT 5;"

-- 10. SELF-LEARNING: Recent outcomes — what worked and what didn't in this area?
-- sqlite3 .claude/memory.db "SELECT agent, score, action, lesson FROM outcomes WHERE domain LIKE '%scheduler%' ORDER BY id DESC LIMIT 15;"

-- 11. SELF-LEARNING: Active lessons — distilled rules for this domain
-- sqlite3 .claude/memory.db "SELECT content, occurrences, confidence FROM lessons WHERE domain LIKE '%scheduler%' AND status='active' ORDER BY confidence DESC;"

-- 12. SELF-LEARNING: Domain learning score — overall effectiveness in this area
-- sqlite3 .claude/memory.db "SELECT domain, avg_score, positive, negative, active_lessons FROM v_domain_learning WHERE domain LIKE '%scheduler%';"

-- 13. Full briefing — composite query for a given scope
-- Run this before any agent starts work. Replace $SCOPE with the area.
.mode column
.headers on

SELECT '=== HOTSPOTS ===' as section;
SELECT file_path, risk_level, times_touched FROM hotspots
WHERE file_path LIKE '%$SCOPE%' ORDER BY times_touched DESC LIMIT 5;

SELECT '=== OPEN FINDINGS ===' as section;
SELECT finding_id, severity, description, file_path FROM findings
WHERE file_path LIKE '%$SCOPE%' AND status='open'
ORDER BY CASE severity WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 WHEN 'P2' THEN 2 WHEN 'P3' THEN 3 END
LIMIT 10;

SELECT '=== FAILED APPROACHES ===' as section;
SELECT approach, failure_reason FROM failed_approaches
WHERE domain LIKE '%$SCOPE%' ORDER BY id DESC LIMIT 5;

SELECT '=== ACTIVE DECISIONS ===' as section;
SELECT decision, rationale FROM decisions
WHERE domain LIKE '%$SCOPE%' AND status='active' ORDER BY id DESC LIMIT 5;

SELECT '=== RECENT BUGS ===' as section;
SELECT description, root_cause FROM bugs
WHERE affected_files LIKE '%$SCOPE%' ORDER BY id DESC LIMIT 5;

SELECT '=== RECENT OUTCOMES (self-learning) ===' as section;
SELECT agent, score, action, lesson FROM outcomes
WHERE domain LIKE '%$SCOPE%' ORDER BY id DESC LIMIT 15;

SELECT '=== ACTIVE LESSONS (self-learning) ===' as section;
SELECT content, occurrences, confidence FROM lessons
WHERE domain LIKE '%$SCOPE%' AND status='active' ORDER BY confidence DESC;
