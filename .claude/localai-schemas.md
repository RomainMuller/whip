# Local AI State Schemas

Reference for `.localai/` directory structure and YAML front-matter schemas.

## Directory Structure

```
.localai/
├── plans/      # High-level implementation plans
├── tasks/      # Granular tasks derived from plans
└── workspaces/ # Isolated workspaces for concurrent agent work
```

## Plans (`.localai/plans/`)

Markdown documents with YAML front-matter for tracking implementation plans.

```yaml
---
id: plan-NNN              # Unique identifier (e.g., plan-001)
title: Short Title        # Human-readable title
status: draft|approved|in_progress|completed|abandoned
created: YYYY-MM-DD
tags: [tag1, tag2]
priority: low|medium|high
estimated_effort: small|medium|large|xlarge
---
```

## Tasks (`.localai/tasks/`)

Granular work items derived from plans, assigned to sub-agents.

```yaml
---
id: task-NNN              # Unique identifier (e.g., task-001)
plan: plan-NNN            # Reference to parent plan
title: Short Title
status: pending|in_progress|completed|blocked
assigned_to: agent-type   # Which agent type handles this
created: YYYY-MM-DD
dependencies: [task-NNN]  # Tasks that must complete first
---
```

Sub-agents update their assigned tasks as they progress.

## Workspaces (`.localai/workspaces/`)

For concurrent agent work. Rules:

1. Retrofit changes as individual commits into root workspace when done
2. Clean up (delete) workspaces after use
3. Commits should be siblings unless dependent on parent changes (use merge commits)
