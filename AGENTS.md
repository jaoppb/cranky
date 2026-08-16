# Cranky Agent Guide (AGENTS.md)

This file provides instructions, architecture overview, and conventions for AI
coding agents contributing to the Cranky codebase.

## 🏗 Architecture Overview

Cranky uses a combination of **Hexagonal Architecture** (Ports and Adapters) and
a **Reactive System** (using Tokio channels and a Signal Hub).

### Directory Structure

Cranky strictly follows **Feature-Sliced Design (FSD)** combined with Clean Architecture principles.

- `src/features/`: Contains domain-specific vertical slices (e.g., `applets`, `layout_engine`, `metrics`, `systray`, `workspaces`, `module_runtime`). Each feature encapsulates its own `domain`, `ports`, and `adapters` related to that business capability.
- `src/shared/`: Unifies cross-cutting concerns, generic infrastructure, and primitive types shared across features (e.g., `config`, `dbus`, `events`, `primitives`, `rendering`, `scripting`, `wayland`).
- `src/app/`: The application entry point and composition layer. It initializes the system, registers actors/builtins, and manages the global state and module registry.

### The Module System

Cranky is entirely modular. Every visual element (`workspace`, `hour`, `applet`,
`metrics`) is an isolated module.

- **Scripting:** Built-in modules are written in either **Lua** or **Rhai**. The
  application detects and runs them using the `ScriptEnginePort`.
- **Reactive Updates:** Modules subscribe to `SignalHub` events (e.g., Time,
  DBus, Metrics) and only re-render when relevant state changes.

## Conventions & Rules

When modifying the codebase, adhere strictly to these principles:

1. **Think Before Coding:** State your assumptions explicitly. If requirements
   are ambiguous, clarify before proceeding.
2. **Simplicity First:** Write the minimum code needed to solve the problem. Do
   not introduce speculative features or over-engineer abstractions for
   single-use code.
3. **Surgical Changes:** Touch only what you must. Do not aggressively refactor
   code adjacent to your task unless it is broken. Match the existing formatting
   and style.
4. **Encapsulation:** Struct fields should remain private; use getter methods to
   expose necessary data.
5. **Error Handling:** Use the `thiserror` crate to define granular, local
   errors within modules and adapters.
6. **Read Before You Write:** Check exports, shared utilities, and caller
   functions before introducing new logic. Avoid silent conflicts; if two
   patterns contradict, pick the better-tested one and explain why.

## 🧪 Testing Requirements

- **Coverage:** Cranky targets an 80%+ unit test coverage for core logic and
  module states.
- **Intent-Driven Tests:** Tests must verify _intent_, not just behavior. Ensure
  your tests fail if the underlying business logic changes incorrectly.
- **Tools:** Use `cargo test` for running tests and `cargo llvm-cov` for
  coverage reporting. New features must include unit tests.

## 🔄 Execution Workflow

1. **Goal-Driven Execution:** Define your success criteria before you start
   typing code. Loop until these criteria are fully verified.
2. **Checkpointing:** After every significant step, summarize what was done,
   what has been verified, and what remains.
3. **Fail Loudly:** Never skip tasks silently. If tests fail or an error occurs,
   surface it immediately rather than masking it.
