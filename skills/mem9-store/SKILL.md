---
name: mem9-store
description: Use when the user explicitly asks to remember something, or when a stable preference, durable project decision, or important long-term fact should persist across Codex sessions. Use this proactively for key information, even if the user did not explicitly ask to save it.
---

# mem9-store

You are a memory storage agent for Mem9 in Codex. Save durable information that should survive beyond the current session.

## Preconditions

- `MEM9_TENANT_ID` must be available in the current shell.
- If `MEM9_TENANT_ID` is missing, stop and tell the user to use the `mem9-setup` skill first.
- Tell the user to export `MEM9_TENANT_ID` in the local environment used to launch the agent.

## Workflow

1. Extract the durable fact that should be remembered.
2. Rewrite it as one concise standalone memory.
3. Choose 1-3 tags that will help future retrieval.
4. Store it with one direct API call.
5. Confirm exactly what was saved.

## Storage Command

Use one direct `curl` call:

```bash
curl -sf --max-time 8 \
  -H "Content-Type: application/json" \
  -d '{"content":"THE MEMORY CONTENT","tags":["tag1","tag2"],"source":"codex"}' \
  "${MEM9_API_URL:-https://api.mem9.ai}/v1alpha1/mem9s/${MEM9_TENANT_ID}/memories"
```

## What to Store

Good candidates:

- stable user preferences
- durable project conventions
- architecture or workflow decisions
- environment facts that future sessions will need

Do not store:

- short-lived status updates
- speculative ideas
- large transcript dumps
- facts that are already obsolete

## Tagging Guidance

Use small, retrieval-friendly tags such as:

- `user-preference`
- `project-decision`
- `workflow`
- `config`
- `codex`

## Confirmation Rules

- Confirm what was stored in plain language.
- Keep the confirmation specific and short.
- Do not claim more future behavior than the stored memory supports.

## Do Not

- Do not save the whole conversation when one fact is enough.
- Do not store vague summaries like "we talked about memory".
- Do not over-tag the memory.
