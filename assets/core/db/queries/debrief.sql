-- ============================================================
-- INCREMENTAL LOGGING — agents run these DURING work
-- ============================================================
-- Log immediately after each significant action. Do NOT batch
-- for the end — if context compacts, batched entries are lost.

-- 1. Start a workflow run (at pipeline start)
-- Returns the run_id to pass to all agents in the chain
-- sqlite3 .claude/memory.db "INSERT INTO workflow_runs (type, description, scope) VALUES ('bugfix', 'fix scheduler crash on empty queue', 'backend/src/scheduler.rs'); SELECT last_insert_rowid();"

-- 2. Log a file change — IMMEDIATELY after modifying the file
-- sqlite3 .claude/memory.db "INSERT INTO changes (run_id, file_path, change_type, description, agent) VALUES (42, 'src/scheduler.rs', 'modified', 'Added null check for empty queue before dequeue', 'developer');"

-- 3. Log a decision — IMMEDIATELY after making it
-- sqlite3 .claude/memory.db "INSERT INTO decisions (run_id, domain, decision, rationale, alternatives, confidence) VALUES (42, 'scheduler', 'Use Option<T> instead of panic on empty queue', 'Panicking crashes the runtime; Option<T> lets callers decide', '[\"sentinel value - rejected: hides errors\", \"Result<T,E> - rejected: not actually an error condition\"]', 0.9);"

-- 4. Log a failed approach — IMMEDIATELY after it fails
-- sqlite3 .claude/memory.db "INSERT INTO failed_approaches (run_id, domain, problem, approach, failure_reason, file_paths) VALUES (42, 'scheduler', 'Empty queue crash', 'Added .is_empty() guard before .pop()', 'Race condition: queue can become empty between check and pop in concurrent context', '[\"src/scheduler.rs\"]');"

-- 5. Log a bug found — IMMEDIATELY after discovering it
-- sqlite3 .claude/memory.db "INSERT INTO bugs (run_id, description, symptoms, root_cause, fix_description, affected_files) VALUES (42, 'Scheduler panics on empty queue', 'Thread panic: called unwrap() on None at scheduler.rs:142', 'dequeue() calls .pop().unwrap() without checking if queue is empty', 'Replaced .pop().unwrap() with .pop() returning Option<Task>', '[\"src/scheduler.rs\", \"src/worker.rs\"]');"

-- 6. Update hotspot counter (upsert) — IMMEDIATELY after modifying a file
-- sqlite3 .claude/memory.db "INSERT INTO hotspots (file_path, times_touched, description, last_incident_run) VALUES ('src/scheduler.rs', 1, 'Scheduler core - concurrent access patterns', 42) ON CONFLICT(file_path) DO UPDATE SET times_touched = times_touched + 1, last_incident_run = 42, last_updated = datetime('now');"

-- 7. Log a finding — IMMEDIATELY after identifying it
-- sqlite3 .claude/memory.db "INSERT INTO findings (run_id, finding_id, severity, category, description, file_path, line_range) VALUES (42, 'AUDIT-P1-003', 'P1', 'bug', 'Race condition in dequeue when multiple workers consume', 'src/scheduler.rs', '140-155');"

-- 8. Mark a finding as fixed — IMMEDIATELY after fixing it
-- sqlite3 .claude/memory.db "UPDATE findings SET status='fixed', fixed_in_run=43 WHERE finding_id='AUDIT-P1-003';"

-- 9. Log a component dependency — IMMEDIATELY after discovering it
-- sqlite3 .claude/memory.db "INSERT OR IGNORE INTO dependencies (source_file, target_file, relationship, discovered_run) VALUES ('src/worker.rs', 'src/scheduler.rs', 'calls', 42);"

-- 10. Log a requirement — IMMEDIATELY after defining it
-- sqlite3 .claude/memory.db "INSERT OR IGNORE INTO requirements (run_id, req_id, domain, description, priority) VALUES (42, 'REQ-SCHED-001', 'scheduler', 'Scheduler must handle empty queue without panicking', 'Must');"

-- 11. Update requirement status — IMMEDIATELY after verification
-- sqlite3 .claude/memory.db "UPDATE requirements SET status='verified', test_ids='[\"TEST-SCHED-001\", \"TEST-SCHED-002\"]' WHERE req_id='REQ-SCHED-001';"

-- 12. Log a discovered pattern — IMMEDIATELY after recognizing it
-- sqlite3 .claude/memory.db "INSERT INTO patterns (run_id, domain, name, description, example_files) VALUES (42, 'scheduler', 'Option-based queue access', 'All queue operations return Option<T> instead of panicking, letting callers handle empty state', '[\"src/scheduler.rs:dequeue\", \"src/scheduler.rs:peek\"]');"

-- ============================================================
-- SELF-LEARNING — score actions IMMEDIATELY, distill at close-out
-- ============================================================

-- 13. Self-score an outcome — IMMEDIATELY after each significant action
-- Score: -1 (approach failed, excessive iteration, suboptimal result)
--         0 (worked but unremarkable, nothing to learn)
--        +1 (clean success, good approach, worth repeating)
-- sqlite3 .claude/memory.db "INSERT INTO outcomes (run_id, agent, score, domain, action, lesson) VALUES (42, 'developer', 1, 'scheduler', 'Used Option<T> for queue access', 'Option<T> pattern avoided unwrap panic that failed_approaches flagged');"

-- ============================================================
-- CLOSE-OUT — run these when agent completes or stops
-- ============================================================

-- 14. Check for lesson distillation opportunity (3+ outcomes with same theme)
-- sqlite3 .claude/memory.db "SELECT domain, score, lesson FROM outcomes WHERE domain = 'scheduler' ORDER BY id DESC LIMIT 10;"
-- Then analyze: do these outcomes suggest a pattern? If yes, INSERT INTO lessons.

-- 15. Distill a lesson from repeated outcomes
-- Content-based dedup: if the lesson already exists, occurrences bumps automatically
-- sqlite3 .claude/memory.db "INSERT INTO lessons (domain, content, source_agent) VALUES ('scheduler', 'Always use Option<T> for container access in concurrent contexts — unwrap causes panics under race conditions', 'developer') ON CONFLICT(domain, content) DO UPDATE SET occurrences = occurrences + 1, confidence = MIN(1.0, confidence + 0.1), last_reinforced = datetime('now');"

-- 16. Reinforce an existing lesson (when you confirm it still holds)
-- sqlite3 .claude/memory.db "UPDATE lessons SET occurrences = occurrences + 1, confidence = MIN(1.0, confidence + 0.1), last_reinforced = datetime('now') WHERE domain = 'scheduler' AND content LIKE '%Option<T>%';"

-- 17. Supersede a lesson (when you discover it no longer applies)
-- sqlite3 .claude/memory.db "UPDATE lessons SET status = 'superseded' WHERE domain = 'scheduler' AND content LIKE '%old pattern%';"

-- 18. Complete a workflow run
-- sqlite3 .claude/memory.db "UPDATE workflow_runs SET status='completed', completed_at=datetime('now'), git_commits='[\"abc1234\", \"def5678\"]' WHERE id=42;"

-- 19. Fail a workflow run
-- sqlite3 .claude/memory.db "UPDATE workflow_runs SET status='failed', completed_at=datetime('now'), error_message='QA iteration limit reached (3/3)' WHERE id=42;"

-- 20. Log decay (for the compressor/maintenance)
-- sqlite3 .claude/memory.db "INSERT INTO decay_log (entity_type, entity_id, action, reason, run_id) VALUES ('decision', 15, 'stale_flagged', 'Referenced file no longer exists after refactor', 42);"
