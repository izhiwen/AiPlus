# /at-status — Show Agent Team status

Use this command to get a one-shot snapshot of the AiPlus Agent Team in
this project.

## How it works

1. Run `aiplus agent status` to surface:
   - Which core roles + experts have been instantiated in
     `.aiplus/agents/` and `.aiplus/agents/experts/`.
   - Which roles have active worktrees (look for `agent/<role>` branches
     or `../<project>.<role>/` directories).
   - Memory namespace sizes per role under
     `.aiplus/agent-memory/<role>/`.
2. Show the current consultant team from `.aiplus/consultant-team.toml`.
3. Surface any pending Owner gates that need attention (production
   deploys, force-push requests, schema migrations, secret rotations).
4. Show in-flight tasks if the team is tracking them via `aiplus agent
   transcript`.

## Examples

```text
/at-status
/at-status ceo            # narrow to just CEO
/at-status experts        # narrow to functional expert table
```

## When to use

- At session start, to see what the team has been doing.
- After a `/clear` or compact, to confirm role state survived.
- Before a major routing decision, to check who has bandwidth.
- When the Owner asks "where are we" — answer from this command's
  output instead of guessing.
