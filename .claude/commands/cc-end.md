# cc-end
Complete feature development with documentation and cleanup.

REQUIRED: Verify all tasks in tasks.md are marked complete
REQUIRED: Validate feature goal from requirements.md has been achieved
REQUIRED: Confirm implementation matches design.md specifications
REQUIRED: Generate comprehensive feature summary at `.specs/{feature_name}/summary.md`
REQUIRED: Archive session.md to feature directory as `session-{date}.md`
REQUIRED: Maintain `.specs/might-it-be.md` — record forward-looking thoughts, future TODOs, and design controversies discovered during implementation
REQUIRED: Maintain `.specs/get-it-done.md` — archive resolved items from might-it-be.md with resolution context
REQUIRED: Clean feature directory — retain only core spec files, remove all auxiliary/working artifacts
REQUIRED: Commit all final changes with completion message
REQUIRED: Clean working directory of temporary files and artifacts outside `.specs/`

PROHIBITED: Completing feature with incomplete tasks
PROHIBITED: Proceeding without goal validation
PROHIBITED: Merging to main branch without user approval
PROHIBITED: Leaving temporary files in working directory
PROHIBITED: Discarding resolved might-it-be items without archiving to get-it-done.md
PROHIBITED: Retaining auxiliary files in feature directory after archival

Feature Directory Retention Policy:
Core files to KEEP in `.specs/{feature_name}/`:
- `requirements.md` — feature requirements definition
- `design.md` — technical design decisions
- `tasks.md` — implementation task tracking (final state)
- `summary.md` — feature completion summary with metrics
- `session-{date}.md` — archived session snapshots

Files to REMOVE (auxiliary/working artifacts):
- `wave.md` — cc-wave execution plan (temporary orchestration artifact)
- `{sub_agent_name}/tasks.md` — sub-agent task tracking (intermediate; final state synced to main tasks.md)
- `{sub_agent_name}/` — entire sub-agent working directories
- Any other files NOT in the retention list above

Removal prerequisite: Before removing wave/sub-agent artifacts, verify:
1. All sub-agent task results have been synced to main `tasks.md` (all tasks marked [x])
2. Main `tasks.md` completion has been validated against `requirements.md` success criteria
3. If either check fails — stop and report, do NOT remove artifacts

Might-It-Be / Get-It-Done Process:
- Scan might-it-be.md (if exists) for items resolved during this feature
- Resolved items → move to get-it-done.md with: original entry, resolution summary, date
- New items discovered during implementation → add to might-it-be.md with:
  - **Forward-looking**: architecture insights, extension points observed during coding
  - **Future TODO**: explicitly deferred work, known tech debt, optimization opportunities
  - **Controversy**: design decisions where trade-offs were debated, alternatives considered
- Each entry format: `## {category}: {title}` + context paragraph
- Present both files to user for review before commit

Completion Sequence:
1. Validate all tasks complete, requirements met, and design implemented
2. Check `.specs/project-info.md` — verify it reflects current feature additions (new modules, API endpoints, dependencies, architecture changes); present update recommendations to user and apply approved changes
3. Maintain might-it-be.md and get-it-done.md — resolve completed items, record new insights
4. Generate comprehensive feature summary with metrics
5. Clean feature directory — remove auxiliary/working artifacts (wave.md, sub-agent dirs, etc.)
6. Archive session.md as session-{date}.md
7. Commit final changes with completion message
8. Clean workspace of temporary files outside `.specs/`
9. Ask user about branch merge/pull request