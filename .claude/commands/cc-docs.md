# cc-docs
Check if project documentation needs updates based on recent changes and present findings for user decision.

REQUIRED: Load skill `arch-design-expert` first to acquire architecture documentation expertise
REQUIRED: Read docs/docs.json to understand current documentation structure and navigation
REQUIRED: Analyze recent code changes (since last docs update) to identify impacted areas
REQUIRED: Compare each doc page content against current implementation state
REQUIRED: Present structured findings with specific update recommendations
REQUIRED: Wait for user decision on which updates to execute
REQUIRED: Only execute updates that user explicitly approves

REQUIRED: All documentation written in Markdown, leverage Mermaid diagrams (flowchart, sequenceDiagram, stateDiagram-v2) for architecture, data flow, state transitions, and module relationships
REQUIRED: Prefer Mermaid diagram over plain text when describing relationships, flows, or hierarchies

PROHIBITED: Auto-updating documentation without user confirmation
PROHIBITED: Skipping docs.json structure analysis
PROHIBITED: Updating docs without verifying code changes justify the update
PROHIBITED: Modifying docs.json navigation without explicit user request
PROHIBITED: Using plain text lists or paragraphs to describe relationships, flows, or hierarchies that could be expressed as Mermaid diagrams

Documentation Check Process:
1. Load docs/docs.json — parse navigation structure, page list, and grouping
2. Identify change scope — scan recent code changes (git diff, uncommitted changes, or user-specified scope)
3. Map changes to docs — correlate code changes to affected documentation pages
4. Audit each affected page — read current doc content and compare against actual implementation
5. Classify update needs:
   - **OUTDATED**: Doc content contradicts current implementation
   - **INCOMPLETE**: New feature/change exists in code but not documented
   - **ORPHANED**: Doc describes removed or non-existent functionality
   - **CURRENT**: Doc accurately reflects implementation (no action needed)
6. Present findings report to user with classification and recommendations
7. Ask user: "Which documentation updates should I execute? Select from recommendations or skip."
8. Execute only user-approved updates

Findings Report Format:
```
## Documentation Audit Report

### Change Scope
{summary of recent code changes analyzed}

### docs.json Navigation
- Navigation structure: {VALID/NEEDS_UPDATE} — {reason if needs update}
- New pages needed: {list or "none"}
- Obsolete pages: {list or "none"}

### Summary
- Total pages checked: N
- Updates needed: N
- Priority updates: {list of OUTDATED items}
```
