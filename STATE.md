# Project Context

**Project:** [Project Name]
**Purpose:** [Brief description of project purpose]

> **Note:** The `hyperi-ai/` submodule provides standards and configuration — not
> code to import. Your project never imports or links to it.
>
> **Behaviour-critical rules** (commit discipline, communication style,
> file discipline, submodule hygiene) live in
> [/hyperi-ai/AGENT-RULES.md](hyperi-ai/AGENT-RULES.md). The table below
> is project-specific file-role guidance, not a duplicate of those rules.

---

## DO NOT ADD TO THIS FILE

**The following belong elsewhere:**

| Data | Correct Location |
|------|------------------|
| Version numbers | `VERSION` file, `git describe --tags` |
| Tasks/Progress | `TODO.md` |
| Session history | Git log (`git log --oneline -10`) |
| Changelog | `CHANGELOG.md` (semantic-release) |
| Dates | Git commit timestamps |

**This file is for static project context only.**

---

## Project Overview

### Architecture

[High-level architecture description]

### Key Components

1. **Component 1** - [Description]
2. **Component 2** - [Description]

### Tech Stack

- **Language:** [Primary language]
- **Framework:** [Framework if applicable]
- **Database:** [Database if applicable]
- **Deployment:** [Deployment method]

---

## Key Decisions

### [Decision Title]

**Decision:** [What was decided]
**Rationale:** [Why this approach was chosen]
**Alternatives considered:** [Other options that were rejected]

---

## External Dependencies

- **[Service/API]** - [What it's used for]
- **[Library]** - [Why it's needed]

---

## Resources

**Documentation:**

- [docs/README.md](docs/README.md) - Project documentation

**External Resources:**

- [External documentation links]
- [API references]

---

## Notes for AI Assistants

This file is **STATE.md**, symlinked as **CLAUDE.md**. It contains shared
project context visible to the whole team. Do not duplicate its contents
into auto-memory — read this file directly instead.

**DO NOT add:**

- Version numbers (use `git describe --tags`)
- Progress/tasks (use `TODO.md`)
- Dates or session history (use `git log`)
- "Current Session" or "Last Session" sections
- Personal preferences (use auto-memory)

**DO add:**

- Architecture decisions and rationale
- Key component descriptions
- External dependencies
- How things work (not what's happening)

When in doubt, ask: "Will this be true next week?" If no, it doesn't belong here.
