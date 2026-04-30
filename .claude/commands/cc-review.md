# cc-review
Review completed tasks against spec documents with structured compliance audit.

REQUIRED: Load skill `arch-design-expert` first to acquire architecture review expertise
REQUIRED: Read all three spec documents (requirements.md, design.md, tasks.md) before review
REQUIRED: Read project-level conventions (AGENTS.md, CLAUDE.md)
REQUIRED: Identify all completed tasks ([x] and [~] marked) as review scope
REQUIRED: Locate implementation code for each completed task via task refs
REQUIRED: Review each completed task against requirements and design specs
REQUIRED: Run project tests and record pass/fail results
REQUIRED: Classify issues by P0/P1/P2 priority with file path and line number
REQUIRED: Present review report to user for confirmation
REQUIRED: Update tasks.md with review findings (mark issues on problematic tasks)

PROHIBITED: Reviewing without reading spec documents first
PROHIBITED: Passing tasks that are marked [x] but incompletely implemented
PROHIBITED: Providing implementation code in review report (report issues only)
PROHIBITED: Ignoring [~] or partially completed tasks
PROHIBITED: Skipping test execution and claiming compliance
PROHIBITED: Outputting metrics statistics or next-step suggestions

Review Dimensions:
- **P0 Functional Compliance**: Implementation faithfully completes spec, no shortcuts/mocks, types match design API contracts, core paths tested
- **P1 Architecture Compliance**: Layer boundaries respected, follows AGENTS.md/CLAUDE.md conventions, SOLID/KISS/DRY/YAGNI adherence, consistent concurrency patterns
- **P2 Code Quality**: Minimal type assertions, clean imports (no circular deps), structured error handling, meaningful logging, no unused imports/variables

Review Process:
1. Load context — read requirements.md, design.md, tasks.md and project conventions
2. Map completion status — extract all [x] and [~] tasks from tasks.md
3. Locate implementation — find source files via task refs and delivery descriptions
4. Sequential compliance audit — verify each completed task against requirements and design
5. Run tests — execute project test command and record results
6. Generate report — organize issues by P0/P1/P2 priority

Report Format:
```
## Review Summary

| Dimension | Status | Issues |
|-----------|--------|--------|
| P0 Functional | PASS/WARN/FAIL | N |
| P1 Architecture | PASS/WARN/FAIL | N |
| P2 Code Quality | PASS/WARN/FAIL | N |

## Issue List

### P0: Functional Compliance
- **`file.ext:L42`** issue description — improvement direction

### P1: Architecture Compliance
- **`file.ext:L42`** issue description — improvement direction

### P2: Code Quality
- **`file.ext:L42`** issue description — improvement direction

## Test Verification
- Command: `{test_command}`
- Result: {passed}/{total} tests passing
- Failures: (if any)

## Conclusion
One-sentence summary of review outcome
```
