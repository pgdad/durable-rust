# Story 6.3: Python-to-Rust Migration Guide

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a senior developer migrating Python durable Lambdas to Rust,
I want a migration guide with conceptual mappings and side-by-side code,
So that I can translate my existing Python patterns to Rust equivalents within 1 week.

## Acceptance Criteria

1. **Given** the docs/migration-guide.md file **When** I read the conceptual mapping section **Then** it provides a table mapping every Python SDK concept to its Rust equivalent **And** covers all 8 core operations plus handler registration, testing, and deployment (FR44)

2. **Given** each core operation in the migration guide **When** I read its section **Then** it shows side-by-side Python and Rust code for the same operation **And** Rust code uses closure-native approach (recommended default)

3. **Given** the migration guide **When** I read the gotchas section **Then** it documents: determinism requirements, Send + 'static bounds, owned data in closures, serde bounds on checkpoint types **And** each gotcha includes concrete "wrong" and "right" code examples

4. **Given** a senior Python developer with some Rust exposure **When** they follow the migration guide end-to-end **Then** they have enough information to migrate an existing Python durable Lambda without reading SDK internals

## Tasks / Subtasks

- [x] Task 1: Create conceptual mapping table (AC: #1)
  - [x] 1.1: Create `docs/migration-guide.md` with header and introduction
  - [x] 1.2: Write mapping table covering: handler registration, step operations, wait, callback, invoke, parallel, map, child context, logging, testing, deployment
  - [x] 1.3: Include Python SDK import patterns vs Rust SDK import patterns

- [x] Task 2: Write side-by-side code for all 8 operations (AC: #2)
  - [x] 2.1: Step (basic) — Python `context.call_activity` vs Rust `ctx.step()`
  - [x] 2.2: Step with retries — Python retry config vs Rust `StepOptions`
  - [x] 2.3: Wait — Python `context.create_wait` vs Rust `ctx.wait()`
  - [x] 2.4: Callback — Python callback pattern vs Rust `ctx.create_callback()` + `ctx.callback_result()`
  - [x] 2.5: Invoke — Python invoke vs Rust `ctx.invoke()`
  - [x] 2.6: Parallel — Python parallel branches vs Rust `ctx.parallel()`
  - [x] 2.7: Map — Python map pattern vs Rust `ctx.map()`
  - [x] 2.8: Child context — Python child context vs Rust `ctx.child_context()`
  - [x] 2.9: Logging — Python logging vs Rust `ctx.log()` family

- [x] Task 3: Write handler registration section (AC: #2)
  - [x] 3.1: Python decorator pattern vs Rust `run()` entry point
  - [x] 3.2: Show all 4 Rust API styles briefly, recommend closure-native as default

- [x] Task 4: Write testing migration section (AC: #1)
  - [x] 4.1: Python mock context vs Rust `MockDurableContext::new().build()`
  - [x] 4.2: Assertion patterns in both languages

- [x] Task 5: Write gotchas section with wrong/right examples (AC: #3)
  - [x] 5.1: Determinism — non-durable code re-executes on replay (wrong: `SystemTime::now()`, right: use event data)
  - [x] 5.2: Send + 'static bounds — parallel/map closures must be sendable (wrong: reference to parent, right: move owned data)
  - [x] 5.3: Owned data in closures — Rust ownership in async closures (wrong: borrow across await, right: clone/move)
  - [x] 5.4: Serde bounds — checkpoint types must be serializable (wrong: non-serializable type, right: derive Serialize)

- [x] Task 6: Write deployment section (AC: #1)
  - [x] 6.1: Container image build (same pattern, different base image)
  - [x] 6.2: Lambda configuration differences

- [x] Task 7: Review and verify (AC: #4)
  - [x] 7.1: Read through as a Python developer — is anything confusing or missing?
  - [x] 7.2: Verify all Rust code examples compile (use `no_run` doc test format)
  - [x] 7.3: `cargo fmt --check` — formatting passes

## Dev Notes

### Document Structure

```
docs/migration-guide.md
├── Introduction
├── Conceptual Mapping Table
├── Handler Registration (Python decorator vs Rust run())
├── Core Operations (8 sections, each with Python-Rust side-by-side)
├── Testing (MockDurableContext vs Python mock)
├── Deployment (Container images)
├── Gotchas (4 sections with wrong/right examples)
└── Quick Reference Card
```

### Python SDK Reference

- GitHub: `aws/aws-durable-execution-sdk-python`
- The Python SDK uses decorators for handler registration and `context.call_activity()` for steps
- Consult the Python SDK docs/examples for accurate Python code in side-by-side comparisons

### Rust Code Style in Guide

All Rust examples should use the closure-native approach (recommended default per Jordan's evaluation in PRD Journey 3). Mention other approaches briefly in the handler registration section.

### Target Audience

Senior Python developer with "some Rust exposure" (PRD Journey 3). Assume they:
- Know Python durable Lambda patterns well
- Have done a Rust tutorial but not production code
- Need conceptual bridges, not Rust basics
- Will use AI coding assistants for implementation

### What Exists

- `docs/` directory exists but may be empty
- No migration guide currently exists

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.3 — acceptance criteria, FR44]
- [Source: _bmad-output/planning-artifacts/prd.md — Journey 3 (Jordan evaluates), Journey 2 (Alex edge case)]
- [Source: _bmad-output/planning-artifacts/architecture.md — docs/migration-guide.md]
- [Source: crates/durable-lambda-closure/src/context.rs — Rust API reference for code examples]

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6 (1M context)

### Debug Log References

None — documentation-only story, no blocking issues.

### Completion Notes List

- Created `docs/migration-guide.md` (~400 lines) covering all 8 core operations with side-by-side Python/Rust code
- Conceptual mapping table covers: handler registration, all 8 operations, logging, testing, deployment, imports
- Handler registration section shows closure-native as default, briefly lists all 4 API styles
- Testing section shows MockDurableContext builder pattern vs Python mock, plus assertion helper comparison table
- Gotchas section covers all 4 required topics with concrete wrong/right code examples
- Deployment section covers Dockerfile differences and Lambda configuration table
- Quick reference card provides at-a-glance cheat sheet
- All Rust code examples use accurate API signatures verified against closure context source
- Formatting passes (`cargo fmt --check`)

### Change Log

- 2026-03-15: Created complete Python-to-Rust migration guide at docs/migration-guide.md

### File List

- docs/migration-guide.md (new — complete migration guide)
