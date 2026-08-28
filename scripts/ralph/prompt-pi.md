# Ralph Agent Instructions

You are an autonomous coding agent working on a software project.

## Your Task

1. Read the PRD at `prd.json` (in the same directory as this file)
   - Also read optional `knownIssues` (array of strings). Treat these as active blockers or gaps.
2. Read the progress log at `progress.txt` (check Codebase Patterns section first)
3. Check you're on the correct branch from PRD `branchName`. If not, check it out or create from main.
4. Pick the **highest priority** user story where `passes: false`.
   - If `knownIssues` exists, prioritize a story that directly resolves one of those issues.
5. Implement that single user story
6. Run quality checks (e.g., typecheck, lint, test - use whatever your project requires)
7. Update AGENTS.md files if you discover reusable patterns (see below)
8. Leave all changes uncommitted for human review. Do not create commits, push, or alter shared Git state.
9. Update the PRD to set `passes: true` for the completed story
   - If the story resolves a `knownIssues` item, remove or rewrite that issue to reflect the new state.
10. Append your progress to `progress.txt`

## Progress Report Format

APPEND to progress.txt (never replace, always append):

If the session has a real share URL, add `Session: <URL>` after the heading.
Omit the session line when no share URL is available.

```text
## [Date/Time] - [Story ID]
- What was implemented
- Files changed
- **Learnings for future iterations:**
  - Patterns discovered (e.g., "this codebase uses X for Y")
  - Gotchas encountered (e.g., "don't forget to update Z when changing W")
  - Useful context (e.g., "the evaluation panel is in component X")
---
```

Note: If session was shared, reference it so future iterations can reference previous work.

The learnings section is critical - it helps future iterations avoid repeating mistakes and understand the codebase better.

## Consolidate Patterns

If you discover a **reusable pattern** that future iterations should know, add it to the `## Codebase Patterns` section at the TOP of progress.txt (create it if it doesn't exist). This section should consolidate the most important learnings:

```text
## Codebase Patterns
- Example: Use `sql<number>` template for aggregations
- Example: Always use `IF NOT EXISTS` for migrations
- Example: Export types from actions.ts for UI components
```

Only add patterns that are **general and reusable**, not story-specific details.

## Update AGENTS.md Files

Before finishing, check if any edited files have learnings worth preserving in nearby AGENTS.md files:

1. **Identify directories with edited files** - Look at which directories you modified
2. **Check for existing AGENTS.md** - Look for AGENTS.md in those directories or parent directories
3. **Add valuable learnings** - If you discovered something future developers/agents should know:
   - API patterns or conventions specific to that module
   - Gotchas or non-obvious requirements
   - Dependencies between files
   - Testing approaches for that area
   - Configuration or environment requirements

**Examples of good AGENTS.md additions:**

- "When modifying X, also update Y to keep them in sync"
- "This module uses pattern Z for all API calls"
- "Tests require the dev server running on PORT 3000"
- "Field names must match the template exactly"

**Do NOT add:**

- Story-specific implementation details
- Temporary debugging notes
- Information already in progress.txt

Only update AGENTS.md if you have **genuinely reusable knowledge** that would help future work in that directory.

## Quality Requirements

- All changes must pass your project's quality checks (typecheck, lint, test)
- Do not leave broken code for review
- Keep changes focused and minimal
- Follow existing code patterns

## Browser Testing (Required for Frontend Stories)

For any story that changes UI, you MUST verify it works in the browser:

1. **Preflight Check**: Look for `chrome-devtools-mcp` in the agent's MCP server configuration.
2. If it is not configured, print:
   ```text
   ⚠️  ChromeDevTools MCP not configured. Frontend testing skipped.
   Configure chrome-devtools-mcp for browser testing:
   https://github.com/ChromeDevTools/chrome-devtools-mcp/
   ```
   Then continue without browser verification.
3. If it is configured, use MCP browser tools to navigate and verify UI changes.
4. Take a screenshot if it helps the progress log.

A frontend story is not complete until browser verification passes, or the preflight confirms that the MCP server is unavailable.

## Stop Condition

After completing a user story, check if ALL stories have `passes: true` and `knownIssues` is empty (or absent).

If ALL stories are complete and passing, reply with:
<promise>COMPLETE</promise>

If there are still stories with `passes: false` OR unresolved `knownIssues`, end your response normally (another iteration will pick up the next story).

## Important

- Work on ONE story per iteration
- Keep each iteration's changes focused and reviewable
- Keep CI green
- Read the Codebase Patterns section in progress.txt before starting
