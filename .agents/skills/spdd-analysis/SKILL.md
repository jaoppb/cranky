---
name: spdd-analysis
description: "Analyze a Cranky requirement (a GitHub issue) against the codebase at a strategic level: domain concepts, strategic direction, risks and Definition of Done gaps, as input for REASONS Canvas generation. Use when starting SPDD work on a requirement that passes the size gate, or when the analysis must be revised. Do not use to design the implementation (use spdd-reasons-canvas) or to write code."
---

# SPDD Analysis

Analyze a requirement against the current codebase and produce a **strategic-level** analysis. It adds domain concept identification, high-level approach decisions and risk analysis to what the requirement already says, and it is the input for `/spdd-reasons-canvas`. This phase covers the "what" and the "why"; the "how" belongs to the REASONS Canvas.

**Input**: a GitHub issue. The issue body is the requirement. `@` file references are reading material to analyse against, not requirement.

```text
/spdd-analysis #95
/spdd-analysis #94 @src/app/bootstrap/error.rs
```

The analysis is written into the issue body under one `## Analysis` heading, never into a file.

## Steps

0. **Resolve the work item (REQUIRED)**

   a. Extract the issue number from the input: `#12`, `GH-12`, or an issue URL.

   b. If no issue is found, ask the user: "Which GitHub issue does this work belong to? (number or URL)". Do NOT proceed without one. Never invent a number or use a placeholder.

   c. Verify it with `gh issue view <n> --json number,title,state,body`. Not found → stop and report. Closed → warn and ask whether to continue. The issue title and body are the requirement.

   d. **Taxonomy.** `AGENTS.md` "Issues, branches and commits" requires exactly one `type:*` label, at least one `area:*` label, the release milestone and an open parent unless the issue carries `epic`. Read them with `gh issue view <n> --json labels,milestone,parent`. Warn, naming the missing field, and ask whether to continue.

   e. **Exemption.** `AGENTS.md` "SPDD" exempts `type:spike` issues: a spike explores, so there is nothing to design yet. Say so and ask whether to continue anyway; never refuse.

   **Size gate** (`AGENTS.md` "SPDD"). SPDD is for issues whose Effort on the Project board is 5 or more. Read the size: `gh project item-list 7 --owner jaoppb --limit 1000 --format json`, then the `effort` of the item whose `content.number` is `<n>`. If the size is below the threshold, is not set, or cannot be read, stop with one short message that says which case applies (together with the exemption question when both apply), and ask "Continue anyway?". Never refuse and never print a token. On yes, continue; on no, stop and change nothing.

   **Board.** Remind the user to move the issue's card to In progress by hand on the Project board (https://github.com/users/jaoppb/projects/7): no `just project-sync` rule moves a card there yet (#97).

1. **Read the requirement**

   a. The requirement (the issue body) is the only source of requirements. If the input adds requirement text that it does not hold, say so and ask the user to add it to the requirement first. Never put that text into the analysis instead.

   b. If the input contains `@` file or folder references, read ALL of them completely (every relevant file in a folder). If any cannot be read, report it and ask for an alternative.

   c. Check that the requirement has a goal, a scope and a Definition of Done. If not, ask the user to complete it first.

2. **Concept-driven codebase exploration**

   Do NOT read the whole codebase. Build a lightweight fingerprint, extract search concepts from the requirement, then explore only the relevant parts in depth.

   a. **Project fingerprint (always first, fast)**:
   - Read `AGENTS.md` (the rules)
   - Read the root `Cargo.toml` (workspace members, `[workspace.dependencies]`, `[lints]`)
   - Read `clippy.toml` (the identifier allow-list, the test-only relaxations, `allowed-duplicate-crates`)
   - Read `arch-lint.toml` (the layer scopes and the rules between them)
   - Read `Justfile` (the `check` gate and the recipes it runs)
   - Skim `tests/architecture.rs` (the structural gate: lint tables, zero exceptions, LOC limits, layer rules)
   - List `src/features/`, `src/shared/`, `src/app/` and `crates/` (directory names only) to see which modules exist

   b. **Extract search concepts** from the requirement:
   - **Domain nouns**: "workspace", "monitor", "special workspace", "tray item", "player", "module", "popup"
   - **Action verbs**: "switch workspace", "render a module", "reload a style", "subscribe to a signal"
   - **Integration surfaces**: Hyprland IPC sockets, D-Bus (MPRIS, StatusNotifierItem), Wayland layer-shell and shm buffers, the Lua and Rhai script API
   - **Technical hints**: "per monitor", "hot reload", "signal subscription", "VNode diff"

   These concepts are the search scope for everything that follows.

   c. **Targeted code exploration**:
   - Search for types, interfaces and modules that match the concepts (e.g. `struct Workspace`, `trait WindowManagerPort`, `SignalKind::Hyprland`, `mod mpris`)
   - Read the matched files to learn the existing invariants, error types, boundaries and adapters
   - Follow direct dependencies **one hop** (read the port an adapter implements or a module calls, and its implementations, but don't keep chaining)
   - Note which module owns each concept and which shared types already exist for reuse
   - Observe conventions in use (naming, error types, test doubles, test names)
   - If no matching code exists, say so explicitly and rely on `AGENTS.md`

   d. **Prior context**:
   - Ask the memory tools (ai-memory `memory_query` and `memory_recent`) for gotchas, decisions and prior sessions on the concepts, and use what they return as hints to check against the code. They are optional: when they are missing, unreachable or empty, say so in one line and continue. Their answers are advisory, and the repository documents and the requirement win on any conflict.

   e. **Controlled expansion (one extra hop only)**: if steps c–d reveal a concept that is clearly essential but was not extracted, add it and do **one** more targeted search. Stop there and note the boundary.

   If the scope turns out very large (10+ existing modules), list all concepts, prioritise the core ones, and mark the rest as "boundary context" to verify during the REASONS Canvas.

3. **Domain Concept Identification**: conceptual level only, with no attributes, types or signatures.

   ```text
   ### Domain Concept Identification

   #### Existing Concepts (from codebase)
   - [ConceptName]: [business purpose] — [where it lives] — [relationship to other concepts]

   #### New Concepts Required
   - [ConceptName]: [business purpose] — [how it relates to existing concepts]

   #### Key Business Rules
   - [Rule, explicit or implicit]: [which concepts it governs]
   ```

4. **Strategic Approach & Trade-offs**: direction, not specifics (no queries, payload shapes, signatures or step-by-step logic).

   - The overall approach, the existing patterns to reuse, and the general data flow (e.g. "Hyprland event socket → `HyprlandAdapter` signal loop → `SignalHub` hyprland channel → each subscribed module's `EventLoop` → script `render(monitor)` → VNode diff → taffy layout → tiny-skia paint → shm buffer")
   - The key decisions: their trade-offs, the recommendation and why

   ```text
   ### Strategic Approach

   #### Solution Direction
   - [High-level description of approach]

   #### Key Design Decisions
   - [Decision]: [trade-offs] → [recommendation and rationale]

   #### Alternatives Considered
   - [Alternative]: [why rejected]
   ```

5. **Risk & Gap Analysis**: surface everything that needs clarifying before detailed design. List only the gaps; never restate the Definition of Done.

   ```text
   ### Risk & Gap Analysis

   #### Requirement Ambiguities
   - [Ambiguity]: [what needs clarification]

   #### Edge Cases
   - [Scenario]: [why it matters]

   #### Technical Risks
   - [Risk]: [potential impact and mitigation direction]

   #### Definition of Done Gaps
   - [Item or scope line]: [what the Definition of Done does not check]
   ```

6. **Assemble** `## Analysis` with the three `###` sections above, in that order. The exploration in step 2 is a working process: its findings show up as file paths and conventions inside the sections, never as a separate "codebase context" section. Never paste what the requirement already says.

7. **Write the section**

   `## Analysis` is always the last section, and nothing above it is ever changed.

   a. Write the body through a temporary file, never an inline string: `gh issue edit <n> --body-file <file>`.

   b. **No `## Analysis` yet**: append it after the existing text, with one blank line between them.

   c. **`## Analysis` already there**: show the user the `###` entries you would add, change or drop, each with a one-line reason. Edit only those and keep the others word for word. After the user confirms, read the live body again (`gh issue view <n> --json body`). If it has changed since you read it, stop and tell the user. Never write over someone else's edit.

   Then show a summary: the work item, the modules involved, the counts of existing concepts, new concepts, key decisions, Definition of Done gaps and open risks, and the next step `/spdd-reasons-canvas #<n>`.

8. **Offer the next phase**: "The analysis is ready. Proceed with `/spdd-reasons-canvas #<n>`?"

## Guardrails

- Do NOT proceed without a resolved work item; never use a placeholder
- Do NOT proceed when the size gate stopped and the user did not confirm
- Do NOT skip codebase exploration, and do NOT read the whole codebase: scope by concepts
- Do NOT assume structure without reading the actual files
- Do NOT paste what the requirement already says (requirement, scope, Definition of Done)
- Use Cranky's vocabulary: feature slices (`src/features/`) and shared units (`src/shared/`), layers (`domain`, `ports`, `adapters`), ports and adapters, the composition root (`src/app/`), the `SignalHub` and its signals (`SignalKind`), modules as actors (`AnyModulePort`, `EventLoop`), Lua and Rhai widget scripts, the render pipeline (VNode diff, taffy layout, tiny-skia paint, shm buffer), newtypes and value types, state keyed by `MonitorId`
- Do NOT include implementation detail (queries, payload shapes, signatures, attributes, step-by-step logic); that belongs to `/spdd-reasons-canvas`
- Do NOT generate code or modify any file in the codebase, and never change the issue body above `## Analysis`
- Do NOT leave placeholders or TODOs
- Definition of Done gaps MUST be listed, and only the gaps; ambiguities MUST be surfaced, never silently assumed
- Read ALL `@` references completely, never truncated; report any that fail

## Where this fits

`/spdd-analysis` (what and why) → `/spdd-reasons-canvas` (how) → `/spdd-generate` (code) → `/spdd-prompt-update` (keep the canvas and the code in step).

| Concern | `/spdd-analysis` | `/spdd-reasons-canvas` |
|---------|------------------|------------------------|
| Thinking level | Strategic: what and why | Tactical: how |
| Domain | Conceptual identification | Detailed entity modelling (E) |
| Solution | Direction and trade-offs | Concrete design (A, S) |
| Implementation | Out of scope | Ordered operations (O) |
| Standards | Out of scope | Norms and safeguards (N, S) |
| Risks | Identify and surface | Resolve through constraints |
