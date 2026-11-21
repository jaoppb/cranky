---
name: spdd-prompt-update
description: "Update an existing Cranky REASONS Canvas with new requirements, architectural changes or corrections, while preserving its structure and cross-section consistency. Use when the design must change before the code does, including after generate or review finds the canvas wrong. Do not use to create a canvas (use spdd-reasons-canvas) or to record changes that were already made in code (rebuild the canvas from the branch diff with spdd-reasons-canvas)."
---

# SPDD Prompt Update

Update an existing REASONS Canvas with new requirements, architectural changes or refinements, keeping its seven-section structure and the quality rules it was generated with.

**Input**: the canvas path and the update instructions.

```text
/spdd-prompt-update @spdd/prompt/GH-<n>-<timestamp>-[Feat]-<scope>-<description>.md
<what to change, e.g. a new constraint, a moved responsibility, a new entity>
```

## Steps

1. **Validate the input**

   a. If there is no canvas path, ask for it. If there are no update instructions, ask what should change. Do NOT proceed without both.

   b. **Resolve the work item (REQUIRED)** from the `GH-<n>-` file-name prefix or the header. If the canvas has none, stop: every canvas belongs to a work item. If the update comes from a different work item, ask whether it should be a new canvas for that item instead. This skill has no size gate.
   Verify the issue with `gh issue view <n> --json number,title,state`.
   The canvas is an ephemeral working file: never stage it, commit it or name it by path (write `#<n>`). If it does not exist, stop and run `/spdd-reasons-canvas #<n>`, which regenerates it.

2. **Read the whole canvas** and identify all seven sections and the current architecture, entities and constraints.

3. **Decide which sections the change affects**

   | Change type | Affected sections |
   |-------------|-------------------|
   | New functional requirement | R, E, A, S, O, possibly N and Safeguards |
   | Architectural change | A, S, O, N |
   | New entity or relationship | E, S, O |
   | New constraint | Safeguards, possibly O |
   | Coding standard change | N, O |
   | Specification bug | The targeted section only |

4. **Read the codebase context the change needs**: the existing shared types and the affected module's domain for new types; `AGENTS.md` for anything architectural; similar existing implementations for new patterns.

5. **Apply the updates**
   - Preserve unchanged content. Don't rewrite sections that need no change.
   - Fit the new content into the existing content, and reflect each change in every related section.
   - Apply the same construction rules as `/spdd-reasons-canvas`. Norms add only change-specific standards, because `AGENTS.md` stays the baseline.

6. **Validate cross-section consistency**: entities used in Operations exist in Entities, the dependencies in Structure match Operations, every Safeguard can be enforced by some Operation, and the Norms are applied in every Operation.

7. **Overwrite the file**, keeping its original file name.

8. **Show a summary**: the changes per section, the sections left unchanged, and the points that need manual review.

9. **Offer to regenerate**: "The canvas is updated. Regenerate the affected code with `/spdd-generate`?"

## Guardrails

- **CRITICAL**: do NOT rewrite the whole file. Modify only the sections the change needs.
- Do NOT proceed without both the canvas path and the instructions, and do NOT update a canvas that has no work item
- Do NOT break cross-section consistency: if Entities changes, check Operations too
- Do NOT leave placeholders or TODOs, and do NOT rename the file
- Keep all seven sections
- Minimal change: keep the original design intent unless the user explicitly asks to change it, and consider the impact on code that already exists

**No code blocks (CRITICAL)**: the canvas is a specification, not source. Describe signatures, query logic and interface contracts in prose with inline code (e.g. `get_state(&self) -> Result<WindowManagerState, WindowManagerError>`). Only Mermaid diagrams are allowed as blocks.

## Common update scenarios

- **New architectural principle**: affects Approach, Structure, Operations, Norms and Safeguards
- **New entity**: affects Entities, Structure and Operations
- **New constraint**: affects Safeguards and possibly Operations
- **Refined business logic**: affects Approach and Operations
- **New coding standard**: affects Norms and the Operations that must follow it
