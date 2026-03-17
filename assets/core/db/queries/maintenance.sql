-- ============================================================
-- MAINTENANCE QUERIES — periodic cleanup and health checks
-- ============================================================

-- 1. Flag stale decisions (referenced domain/files changed significantly)
-- Run periodically or at workflow start
UPDATE decisions SET status = 'stale'
WHERE status = 'active'
AND id IN (
    SELECT d.id FROM decisions d
    WHERE NOT EXISTS (
        SELECT 1 FROM changes c
        WHERE c.file_path LIKE '%' || d.domain || '%'
        AND c.created_at > d.created_at
    )
    AND d.created_at < datetime('now', '-30 days')
);

-- 2. Promote hotspots based on incident frequency
UPDATE hotspots SET risk_level = 'critical'
WHERE times_touched >= 10 AND risk_level != 'critical';

UPDATE hotspots SET risk_level = 'high'
WHERE times_touched >= 5 AND times_touched < 10 AND risk_level NOT IN ('critical', 'high');

UPDATE hotspots SET risk_level = 'medium'
WHERE times_touched >= 3 AND times_touched < 5 AND risk_level NOT IN ('critical', 'high', 'medium');

-- 3. Summary stats
SELECT '=== MEMORY HEALTH ===' as report;
SELECT 'Workflow runs' as metric, COUNT(*) as value FROM workflow_runs
UNION ALL SELECT 'Completed', COUNT(*) FROM workflow_runs WHERE status='completed'
UNION ALL SELECT 'Failed', COUNT(*) FROM workflow_runs WHERE status='failed'
UNION ALL SELECT 'Open findings', COUNT(*) FROM findings WHERE status='open'
UNION ALL SELECT 'P0 open', COUNT(*) FROM findings WHERE status='open' AND severity='P0'
UNION ALL SELECT 'P1 open', COUNT(*) FROM findings WHERE status='open' AND severity='P1'
UNION ALL SELECT 'Failed approaches logged', COUNT(*) FROM failed_approaches
UNION ALL SELECT 'Active decisions', COUNT(*) FROM decisions WHERE status='active'
UNION ALL SELECT 'Hotspots tracked', COUNT(*) FROM hotspots
UNION ALL SELECT 'Critical hotspots', COUNT(*) FROM hotspots WHERE risk_level='critical'
UNION ALL SELECT 'Bugs logged', COUNT(*) FROM bugs
UNION ALL SELECT 'Patterns discovered', COUNT(*) FROM patterns;

-- 4. Top 10 hottest files
SELECT file_path, risk_level, times_touched,
    (SELECT COUNT(*) FROM findings f WHERE f.file_path = h.file_path AND f.status='open') as open_findings,
    (SELECT COUNT(*) FROM bugs b WHERE b.affected_files LIKE '%' || h.file_path || '%') as bug_count
FROM hotspots h
ORDER BY times_touched DESC
LIMIT 10;

-- 5. Decay: archive old resolved findings (older than 90 days)
INSERT INTO decay_log (entity_type, entity_id, action, reason)
SELECT 'finding', id, 'archived', 'Resolved more than 90 days ago'
FROM findings
WHERE status IN ('fixed', 'wontfix')
AND created_at < datetime('now', '-90 days')
AND id NOT IN (SELECT entity_id FROM decay_log WHERE entity_type='finding' AND action='archived');

-- 6. Orphaned hotspots — files that no longer exist could be flagged
-- (Agents should run: for each hotspot, check if file exists, if not flag it)
SELECT file_path FROM hotspots
WHERE last_updated < datetime('now', '-60 days');

-- ============================================================
-- SELF-LEARNING MAINTENANCE
-- ============================================================

-- 7. Enforce lesson cap: max 10 active lessons per domain
-- Keeps knowledge fresh by pruning oldest when cap is exceeded
-- Archive excess lessons (keeps the 10 highest-confidence per domain)
UPDATE lessons SET status = 'archived'
WHERE status = 'active'
AND id NOT IN (
    SELECT id FROM lessons l2
    WHERE l2.domain = lessons.domain AND l2.status = 'active'
    ORDER BY l2.confidence DESC, l2.occurrences DESC
    LIMIT 10
);

INSERT INTO decay_log (entity_type, entity_id, action, reason)
SELECT 'lesson', id, 'archived', 'Lesson cap exceeded (>10 per domain) — lowest confidence pruned'
FROM lessons
WHERE status = 'archived'
AND id NOT IN (SELECT entity_id FROM decay_log WHERE entity_type='lesson' AND action='archived');

-- 8. Archive old outcomes (keep last 60 days of raw scores)
-- Outcomes older than 60 days have already been distilled into lessons (hopefully)
DELETE FROM outcomes
WHERE created_at < datetime('now', '-60 days');

-- 9. Decay lesson confidence for unreinforced lessons (not reinforced in 30+ days)
UPDATE lessons SET confidence = MAX(0.1, confidence - 0.1)
WHERE status = 'active'
AND last_reinforced < datetime('now', '-30 days');

-- 10. Archive zero-confidence lessons
UPDATE lessons SET status = 'archived'
WHERE status = 'active'
AND confidence <= 0.1
AND last_reinforced < datetime('now', '-60 days');

-- 11. Self-learning health stats
SELECT '=== SELF-LEARNING HEALTH ===' as report;
SELECT 'Total outcomes' as metric, COUNT(*) as value FROM outcomes
UNION ALL SELECT 'Positive outcomes (+1)', COUNT(*) FROM outcomes WHERE score = 1
UNION ALL SELECT 'Neutral outcomes (0)', COUNT(*) FROM outcomes WHERE score = 0
UNION ALL SELECT 'Negative outcomes (-1)', COUNT(*) FROM outcomes WHERE score = -1
UNION ALL SELECT 'Overall avg score', ROUND(AVG(score), 2) FROM outcomes
UNION ALL SELECT 'Active lessons', COUNT(*) FROM lessons WHERE status = 'active'
UNION ALL SELECT 'Archived lessons', COUNT(*) FROM lessons WHERE status = 'archived'
UNION ALL SELECT 'Strong lessons (5+ occurrences)', COUNT(*) FROM lessons WHERE status = 'active' AND occurrences >= 5
UNION ALL SELECT 'Domains with lessons', COUNT(DISTINCT domain) FROM lessons WHERE status = 'active';

-- 12. Learning effectiveness per domain
SELECT domain, total_outcomes, positive, negative, avg_score, active_lessons
FROM v_domain_learning
ORDER BY total_outcomes DESC
LIMIT 10;
