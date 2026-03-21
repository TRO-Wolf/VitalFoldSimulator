## Workflow Orchestration

### 1. Plan Mode Default
- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- Write the plan to 'task/todo.md' BEFORE writing any code
- If something goes sideways, STOP and re-plan immediately — don't keep pushing
- Use plan mode for verification steps, not just building
- When something is ambiguous, ask for clarification before proceeding
- Re-read 'task/todo.md' AND 'task/lessons.md' before EVERY implementation step, not just at session start
- If a plan step reveals unexpected complexity, update the plan with sub-steps before continuing

### 2. Self-Improvement Loop
- After ANY correction from the user: update 'task/lessons.md' with the pattern immediately
- Write rules as concrete DO or DO NOT statements with brief context or examples
- Ruthlessly iterate on these lessons until the mistake rate drops
- At the start of every session, read 'task/lessons.md' in full before doing anything else
- Review lessons before each implementation step, not just at session start
- NEVER use placeholders like `// rest of code`, `...`, or `# existing code unchanged` — write complete functions. If a function is too long for one response, say so explicitly and split across responses with each section complete

### 3. Context & File Awareness
- Before editing ANY file, re-read it first — do not rely on your memory of its contents from earlier in the conversation
- After making edits, re-read the modified file to confirm the change landed correctly and did not corrupt surrounding code
- When a conversation grows long, proactively re-read files you are about to modify
- Never assume you know the current state of a file — always verify before writing

### 4. Verification Before Done — Task is NOT done until all boxes are checked
- [ ] Code compiles / interprets without errors (run it, do not assume)
- [ ] Tests pass (if no tests exist, write at least one happy-path and one edge-case test)
- [ ] Output matches expected schema or contract
- [ ] Null/empty/edge cases are handled
- [ ] No new warnings or errors in logs
- [ ] No unintended changes outside the target files
- [ ] Logic has been traced manually or via a test for at least one representative input
- [ ] Imports and dependencies are correct and actually used — no orphaned imports
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer at a quant hedge fund approve of this?"
- Never mark a task complete without proving it works

### 5. Demand Elegance (Balanced)
- For non-trivial changes: pause and ask "is there a more elegant way?"
- If a fix feels hacky: step back and implement the clean solution with full context
- Skip this for simple, obvious fixes — don't over-engineer
- Challenge your own work before presenting it
- Prefer boring, obvious code over clever solutions — elegance means clarity, not complexity

### 6. Scope Boundaries — Hard Rules
- Only modify files explicitly listed in the current plan
- Do not rename, reorganize, or clean up unrelated code even if it looks wrong
- If a fix requires touching an unexpected file, STOP and check in first
- Do not add features, refactors, or "improvements" the user did not ask for
- Do not change function signatures, return types, or class interfaces unless the plan explicitly calls for it

### 7. Dependency & API Rules
- Before writing any code using an external library, verify the API is current and not deprecated
- Libraries to always verify: Polars, DuckDB, Apache Arrow, Apache Iceberg, Airflow, PySpark
- If there is a difference between what you originally intended to write and what is currently found in the library, record the correct usage in 'task/lessons.md'
- Plan for Apache Arrow columnar format in the long term — Parquet, OLAP, and the like
- Do NOT modify requirements.txt, pyproject.toml, Cargo.toml, or any dependency file without explicit approval
- When using a library function, use the exact method signature — do not guess parameter names or assume default behavior

### 8. Debugging Protocol — Follow in order, do not skip steps
1. **Read the actual error** — Copy the full error message; do not guess from a summary
2. **Reproduce** — Confirm you can trigger the error consistently
3. **Isolate** — Identify the exact file, function, and line
4. **Hypothesize** — State one specific cause BEFORE changing anything
5. **Fix** — Make the smallest change that addresses the hypothesis
6. **Verify Fix** — Confirm the hypothesis was correct after the fix
7. **Check for Regression** — Run existing tests; confirm nothing else broke

- Never refactor code outside of the files directly related to the task
- One change at a time — do not bundle multiple fixes in a single edit
- If the same error persists after two fix attempts, STOP, re-read the relevant code from disk, and re-assess from scratch rather than layering more patches

### 9. Code Quality Gates
- No magic numbers — use named constants or configuration values
- Every function must have a docstring or comment stating what it does, what it takes, and what it returns
- Error messages must be specific and actionable — not generic "something went wrong"
- Use type hints in Python; use explicit types in Rust — do not leave types inferred where clarity matters
- If copying logic from one place to another, extract it into a shared function instead


## Task Management
1. **Plan First**: Write plan to 'task/todo.md' with checkable items
2. **Verify Plan**: Check in with the user before starting implementation
3. **Track Progress**: Mark items complete as you go, keep the document concise for context sizing
4. **Explain Changes**: One-sentence summary per step — what changed and why
5. **Document Results**: Add review section to 'task/todo.md' when done
6. **Capture Lessons**: Update 'task/lessons.md' after any correction
7. **Think Before Acting**: Before any major logic block, stop and reason through the approach — what are the inputs, what are the edge cases, what could go wrong? Do this BEFORE writing code


## Core Principles
- **Simplicity First**: Make every change as simple as possible. Minimize code impact
- **Small Functions**: Keep functions under 100 lines; one function = one responsibility
- **No Laziness**: Find root causes. No temporary fixes. Senior developer and production standards
- **Minimal Impact**: Changes should only touch what's necessary. If in doubt, do less and ask
- **No Assumptions**: If something is not explicitly stated in the plan, ask before acting
- **Read Before Write**: Always read the current file state before making any edit
- **Fail Loudly**: If you are unsure about something, say so immediately — do not silently guess and hope for the best
