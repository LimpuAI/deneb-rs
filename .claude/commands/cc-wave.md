# cc-wave
Execute multiple tasks in parallel waves with dependency analysis, visual plan generation, and agent dispatch.

REQUIRED: Read tasks.md to identify all uncompleted tasks and their dependencies
REQUIRED: Read referenced requirements.md and design.md sections for task context
REQUIRED: Build dependency graph and group tasks into parallel execution waves
REQUIRED: Maintain `.specs/{feature_name}/wave.md` — create on first run, load and update on subsequent runs
REQUIRED: Analyze each task's characteristics to select appropriate agent type
REQUIRED: Dispatch multiple agents concurrently within each wave
REQUIRED: Monitor agent execution and collect results
REQUIRED: Maintain `.specs/{feature_name}/{sub_agent_name}/tasks.md` for agents handling multiple tasks — create before dispatch, update on completion, load on resume
REQUIRED: Update tasks.md status as agents complete their tasks
REQUIRED: Present wave results to user for confirmation before next wave
REQUIRED: Update Progress section only after user confirms wave completion

PROHIBITED: Dispatching tasks with unresolved dependencies
PROHIBITED: Executing multiple waves without user confirmation between waves
PROHIBITED: Marking tasks complete without agent confirmation and test verification
PROHIBITED: Auto-proceeding to next wave after completion
PROHIBITED: Skipping progress updates in tasks.md
PROHIBITED: Executing without active wave.md (create new or load existing)
PROHIBITED: Dispatching multi-task agents without creating their sub-agent tasks.md first

Dependency Analysis:
1. Parse all uncompleted tasks from tasks.md (status: [ ])
2. Extract explicit dependencies from task descriptions (e.g., "depends on task N")
3. Infer implicit dependencies from shared files, data flow references, and phase ordering
4. Build directed acyclic graph (DAG) of task dependencies

Wave Grouping Rules:
- Tasks with no dependencies on other uncompleted tasks → Wave 1
- Tasks whose dependencies are all in earlier waves → eligible for current wave
- Tasks with cross-dependencies or shared mutable files → same wave (sequential within)
- Maximum wave size: ≤ 4 concurrent agents

Agent Selection Strategy:
- Analyze task characteristics (language, domain, file types, complexity)
- Select agent type based on task profile:
  - `typescript-expert`: TypeScript/ frontend logic, type system design, Svelte components
  - `rust-expert`: Rust backend, ownership model, async runtime, trait design
  - `general-purpose`: Mixed tasks, configuration, documentation, integration work
  - `Explore`: Research-heavy tasks requiring codebase investigation
- Provide each agent with: task description, relevant spec excerpts, target files, dependencies

Execution Flow:
1. Load tasks.md and analyze dependency graph
2. Maintain wave.md:
   - If `.specs/{feature_name}/wave.md` exists → load existing plan, skip completed waves, resume from current
   - If not exists → create new wave.md with visual execution plan (see Wave Plan Template below)
3. Present execution plan to user: wave breakdown, agent assignments, parallelism estimate
4. Ask user: "Confirm execution plan? Adjust wave grouping or agent selection?"
5. ONLY after user confirms: Begin Wave 1 execution
6. For each wave:
   a. For each agent in this wave:
      - If single task → no sub-agent tasks.md needed, dispatch directly
      - If multiple tasks → create `.specs/{feature_name}/{sub_agent_name}/tasks.md` (see Sub-Agent Tasks Template below); if already exists from prior run → load and skip completed sub-tasks
   b. Dispatch agents concurrently (one agent per task group)
   c. Each agent receives: task spec, relevant requirements/design excerpts, target files, and path to its sub-agent tasks.md if applicable
   d. Monitor agent progress and collect completion status; agents update their own tasks.md as they complete sub-tasks
   e. After all agents in wave complete: run tests for changed files
   f. Present wave results to user with implementation evidence
   g. Ask user: "Does this wave meet requirements? Confirm to proceed?"
   h. ONLY after user confirms: Update main tasks.md [x] marks and Progress section
   i. Update wave.md with completion status
   j. Proceed to next wave
7. After all waves complete: present final summary
8. Suggest running cc-review for comprehensive compliance audit

Wave Plan Template — save to `.specs/{feature_name}/wave.md`:
```
# Wave Execution Plan

Generated: {timestamp}
Feature: {feature_name}
Total tasks: {total} | Pending: {pending} | Waves: {wave_count}

## Dependency Graph

{ASCII diagram showing task dependency DAG}

## Wave Schedule

┌────────────────────────────────────────────────────────────────────┐
│  WAVE EXECUTION                                                    │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  WAVE 1 (parallel)          WAVE 2 (parallel)          WAVE 3      │
│  ┌─────────┐ ┌─────────┐    ┌─────────┐ ┌─────────┐    ┌─────────┐ │
│  │ Task 01 │ │ Task 02 │ →  │ Task 03 │ │ Task 04 │ →  │ Task 05 │ │
│  │         │ │         │    │         │ │         │    │         │ │
│  │ {desc}  │ │ {desc}  │    │ {desc}  │ │ {desc}  │    │ {desc}  │ │
│  │ {agent} │ │ {agent} │    │ {agent} │ │ {agent} │    │ {agent} │ │
│  └─────────┘ └─────────┘    └─────────┘ └─────────┘    └─────────┘ │
│       │           │              ↑           ↑              ↑      │
│       └───────────┘──────────────┘           │              │      │
│              Deps: 03←01, 04←02             │              │      │
│                                    ┌────────┘──────────────┘      │
│                                    Deps: 05←03+04                 │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘

## Wave Details

### Wave 1 — {status: pending/running/done}
| Task | Title | Agent | Files | Status |
|------|-------|-------|-------|--------|
| 01 | {title} | {agent_type} | {files} | ⬜ pending |
| 02 | {title} | {agent_type} | {files} | ⬜ pending |

### Wave 2 — {status: pending/running/done}
| Task | Title | Agent | Files | Status |
|------|-------|-------|-------|--------|
| 03 | {title} | {agent_type} | {files} | ⬜ pending |
| 04 | {title} | {agent_type} | {files} | ⬜ pending |

### Wave 3 — {status: pending/running/done}
| Task | Title | Agent | Files | Status |
|------|-------|-------|-------|--------|
| 05 | {title} | {agent_type} | {files} | ⬜ pending |

## Execution Log
| Time | Wave | Event | Detail |
|------|------|-------|--------|
| {ts} | W1 | started | Wave 1 dispatched |
| {ts} | W1 | task.done | Task 01 completed |
```

Agent Prompt Template:
```
You are executing task {N}: {task_title}

Context:
- Requirements: {relevant_requirements_excerpt}
- Design: {relevant_design_excerpt}
- Dependencies completed: {completed_dependency_list}

Target files: {file_list}
Constraints: {task_specific_constraints}

{sub_agent_tasks_section}

Implement this task faithfully according to the spec. Do not simplify, mock, or skip.
After implementation, verify your work against the requirements.
```

Sub-Agent Tasks Section (inject into prompt when agent handles multiple tasks):
```
You are responsible for multiple tasks. Track your progress in:
`.specs/{feature_name}/{sub_agent_name}/tasks.md`

Read this file first — if it exists, resume from where you left off (skip completed items).
After completing each task, immediately update the file with [x] mark.
Do NOT wait until all tasks are done to update.
```

Sub-Agent Tasks Template — save to `.specs/{feature_name}/{sub_agent_name}/tasks.md`:
```
# {sub_agent_name} Tasks

Parent wave: W{N}
Agent type: {agent_type}
Status: {pending/running/done}

## Tasks
- [ ] {main_task_id}. {task_title} — {file_list}
- [ ] {main_task_id}. {task_title} — {file_list}
- [ ] {main_task_id}. {task_title} — {file_list}

## Notes
{implementation_discoveries}
```

NOTE: Task numbering MUST match the main tasks.md exactly (e.g., if main tasks.md has tasks 52, 53, 54, sub-agent tasks.md uses 52, 53, 54 — never renumber to 1, 2, 3).
