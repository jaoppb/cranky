---
name: spdd-generate
description: "Generate Cranky code and tests from a REASONS Canvas structured prompt, executing its Operations in order under its Norms and Safeguards. Use when a canvas from spdd-reasons-canvas is confirmed and ready to implement, or to regenerate the code affected by a canvas update. Do not use without a canvas (use spdd-reasons-canvas), and do not use it to patch code that diverged from the canvas (update the canvas first with spdd-prompt-update)."
---

# SPDD Generate

Generate implementation code from a structured SPDD prompt (a REASONS Canvas). Follow its Operations sequence, Norms and Safeguards strictly, within Cranky's rules (`AGENTS.md`).

**Input**: the canvas path, e.g. `@spdd/prompt/GH-<n>-<timestamp>-[Feat]-<scope>-<description>.md`.

## Steps

1. **Get the canvas.** If no path was given, ask the user for one. Do NOT proceed without a valid canvas.

2. **Resolve the work item (REQUIRED)**
   - Take it from the `GH-<n>-` file-name prefix (or the header line). If the canvas has none, stop: fix the canvas first with `/spdd-prompt-update`.
   - Verify it with `gh issue view <n> --json number,title,state`. Not found → stop. Closed → warn and ask.
   - The canvas is an ephemeral working file: never stage it, commit it or name it by path in a commit, issue or pull request (write `#<n>`). If it does not exist, stop and run `/spdd-reasons-canvas #<n>`, which regenerates it from the analysis and the code already written.

3. **Read the whole canvas** and extract its sections:

   | Section | Usage |
   |---------|-------|
   | **R** Requirements | Business context, Definition of Done, out of scope |
   | **E** Entities | Types, stereotypes, placement |
   | **A** Approach | Strategy, errors, tests |
   | **S** Structure | slices, modules, implementations, dependencies, boundaries |
   | **O** Operations | **Execute in the defined order** |
   | **N** Norms | Change-specific standards on top of `AGENTS.md` |
   | **S** Safeguards | Non-negotiable constraints, exact error messages |

4. **Load the project rules and context**: `AGENTS.md` (the baseline norms every canvas inherits), the tool configs below, and the modules named in Structure together with their closest existing patterns.
   - Read the root `Cargo.toml` (workspace members, `[workspace.dependencies]`, `[lints]`)
   - Read `clippy.toml` (the identifier allow-list, the test-only relaxations, `allowed-duplicate-crates`)
   - Read `arch-lint.toml` (the layer scopes and the rules between them)
   - Read `Justfile` (the `check` gate and the recipes it runs)
   - Skim `tests/architecture.rs` (the structural gate: lint tables, zero exceptions, LOC limits, layer rules)

   Generated code MUST follow the existing conventions.

5. **Validate the Operations sequence**
   - The dependency order is right (shared value types → slice domain → ports → adapters → script API (Lua and Rhai together) → `src/app/` wiring → widgets and styles → boundaries → docs → verify), and no operation uses a type defined by a later one
   - No dependency breaks the boundary rules:
     - `domain` and `ports` never import `adapters` or `app`; `features` and `shared` never import `app` (`arch-lint.toml`, checked by `arch_lint::check!()` in `tests/architecture.rs`)
     - `src/app/` is the composition root: it builds the adapters, wires them to the ports and starts them; nothing depends on it
     - `unsafe` code lives only in `crates/cranky-unsafe`; the `cranky` crate forbids `unsafe_code`
     - A change to the scripting API is made in both the Lua and the Rhai adapters (`src/shared/scripting/adapters/`) and covered by the same tests in `src/app/builtins/tests.rs`
     - Visibility follows `AGENTS.md` "Public surface": an item is only as visible as its uses
   - Each operation is atomic, names its slice and module, and lists its tests. Together they cover all of Structure, and they match its implementations, dependencies and boundary rules.

   If you find a problem, report it and suggest a `/spdd-prompt-update` before proceeding. Do NOT re-plan the sequence: the Operations order is the designed order.

6. **Generate the code, one operation at a time, in order**

   a. Read the operation spec: responsibility, representation, signatures, errors, behaviour and tests.

   b. **Apply the Norms** (`AGENTS.md` and the canvas Norms):
   - `AGENTS.md` "Hard constraints" is the full list; these are the rules generation breaks most often
   - Private fields with getters; newtypes for ids and names; `thiserror` errors per module or adapter
   - No `as`, unchecked arithmetic or string slicing anywhere, tests included; no `unwrap`/`expect`/`panic!`/indexing outside tests (`clippy.toml` relaxes those, and `print`/`dbg`, in tests): use `get()`, `TryFrom`, `checked_*`/`saturating_*` and `Result`
   - Identifiers of three or more characters, except `x`, `y`, `id`, `tx`, `rx`, `dx`, `dy` and `px`, tests included; name a binding after what it holds
   - Each item only as visible as its uses (`pub(super)`, `pub(in crate::…)`, `pub(crate)` only inside a `pub` module chain); every exported item has a doc that states its intent, and every exported type a `Debug` impl
   - No `#[tokio::test]`, `#[tokio::main]`, `#[tracing::instrument]` or `mockall`: tests use `crate::test_utils::block_on`, ports get hand-written fakes, spans are explicit
   - `#[must_use]` on constructors and getters, `const fn` where Clippy asks for it
   - At most 200 production LOC per file under `src/features/` and `src/shared/`: split into submodules, never allowlist
   - CSS classes are not prefixed with the module name (`.container`, `.item`)
   - A scripting API change goes into both the Lua and the Rhai adapter

   c. **Enforce the Safeguards**:
   - Use the exact error names and messages (do not modify them)
   - Apply the domain, data and security constraints exactly as written
   - **Zero lint exceptions**: never write `#[allow(...)]`, `#[expect(...)]` or `#![allow(...)]`, and no macro that emits one (`#[tokio::test]`, `#[tokio::main]`, `#[tracing::instrument]`, `mockall`). If code cannot fit a rule, stop and report it. The fix is the canvas, or the rule changed in `Cargo.toml` for everyone (`AGENTS.md`), never an exemption.

   d. **Write the tests the operation lists**:
   - Unit tests live next to the code in `#[cfg(test)] mod tests` (or a `tests.rs`/`tests/` submodule); shared helpers are in `src/test_utils.rs`
   - Async code is driven by `crate::test_utils::block_on`, never `#[tokio::test]`
   - Ports are replaced by hand-written fakes (`Fake*`), never mockall; assert on behaviour, not call counts
   - A test fails when the logic it covers changes incorrectly
   - A D-Bus or Wayland adapter change also gets an integration test in `tests/integration/` (private `dbus-daemon`, headless `sway`, `grim`)
   - Line coverage stays at 80% or more (`just coverage-check`)

   e. Write the code in the slice and module the operation names.

   Do NOT deviate from the Operations, add features, methods or fields that are not specified, or change the Safeguard messages.

7. **Batch validation**

   After ALL the code is generated, run `just check` (fmt-check, lint, doc-check, arch-check, loc-check, test, deps-check and workflows-check; add `just coverage-check` and `just bench-check` when the change can affect coverage or the benchmarks).

   a. Fix failures by changing the code to fit the rule, never by adding exceptions.

   b. Cross-check every Definition of Done item, and check that the error names and messages match the Safeguards exactly.

   c. Check that placement matches Structure, that no forbidden dependency was introduced, and that `grep -rnE '#!?\[(allow|expect)\(' src tests benches crates` prints nothing.

   If a failure cannot be fixed without deviating from the canvas, stop and propose a `/spdd-prompt-update`.

8. **Report**: the work item and the canvas, the files created or changed and their responsibilities, any deviations or assumptions, the `just check` result, and the Definition of Done checklist (pass or fail per item).
   **Board:** after the pull request opens, and again after the merge, run `just project-check`, then `just project-sync` (it needs the `project` scope: `gh auth refresh -s project`).
   The pull request body names the issue with `Closes #<n>`.

## Review and iteration loop

> **When reality diverges, fix the prompt first, then update the code.**

If the code was already changed by hand (a review fix) and more operations remain, first bring the canvas in line with it (`/spdd-reasons-canvas #<n>`, stale canvas), so that generation does not undo the change.

1. Identify the incorrect or missing behaviour.
2. Trace it to a section: wrong interpretation → Requirements; missing type or relationship → Entities; flawed strategy → Approach; wrong placement → Structure; wrong detail → Operations; missing standard → Norms; missing constraint → Safeguards.
3. Update the canvas with `/spdd-prompt-update`.
4. Regenerate only the affected components.
5. Commit the code and its tests, referencing the issue (`Refs #<n>` in commits; only the pull request body says `Closes #<n>`). The canvas is never part of a commit.

## Guardrails

- Do NOT generate code before reading the entire canvas and `AGENTS.md`
- Do NOT generate code for a canvas without a resolved work item
- Do NOT re-plan, reorder or skip operations
- Do NOT change signatures, field names, variant names or error messages from the specification
- Do NOT add features, types or fields that are not specified
- Do NOT add lint exceptions of any kind
- Do NOT patch code directly when issues are found: update the canvas first
- Always verify against the Definition of Done, and always finish with `just check` passing
- Never stage the canvas: commit only code and tests
- When generating reveals a trap too small for a rule in `AGENTS.md`, put it in a code comment if later changes must obey it, and otherwise in the step 8 report. The memory tools are read-only here and optional: if they are unavailable, say so in one line and continue.
