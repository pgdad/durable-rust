# Story 1.2: Core Types & Error Foundation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want typed SDK errors and core data types,
So that all SDK components share a consistent type system and error handling from the start.

## Acceptance Criteria

1. **Given** the durable-lambda-core crate **When** I examine the types module **Then** it exports `HistoryEntry` (representing a single checkpoint record), `ExecutionMode` (`Replaying { history, cursor } | Executing`), and `CheckpointResult<T, E>` (`Ok(T) | Err(E)`) **And** all types implement `Serialize + DeserializeOwned` via serde

2. **Given** the durable-lambda-core crate **When** I examine the error module **Then** it exports a `DurableError` enum with thiserror derives **And** `DurableError` has variants for replay mismatch, checkpoint failure, serialization errors, and AWS SDK errors **And** each variant is constructed via constructor methods (e.g., `DurableError::replay_mismatch(...)`), never raw struct construction **And** variants wrapping underlying errors use `#[from]` or `source()` for error chain propagation

3. **Given** the `DurableError` type **When** an AWS SDK error occurs **Then** it is propagated through `DurableError` with full context preserved (FR49)

4. **Given** the `DurableError` type **When** a serde serialization/deserialization error occurs **Then** it is propagated through `DurableError` with type name and source error (FR50)

5. **Given** all public types and error variants **When** I run `cargo test --doc -p durable-lambda-core` **Then** all rustdoc examples compile and pass

## Tasks / Subtasks

- [x] Task 1: Implement core types in `types.rs` (AC: #1)
  - [x] 1.1: Define `HistoryEntry` struct with fields for operation name, serialized result (as `serde_json::Value`), and operation type — derive `Serialize, Deserialize, Debug, Clone, PartialEq`
  - [x] 1.2: Define `ExecutionMode` enum with two variants: `Replaying` (no fields — history/cursor live in the replay engine, not this enum) and `Executing` — derive `Debug, Clone, PartialEq`
  - [x] 1.3: Define `CheckpointResult<T, E>` as an enum `Ok(T) | Err(E)` — derive `Serialize, Deserialize, Debug, Clone, PartialEq` with serde bounds on T and E
  - [x] 1.4: Add rustdoc with `# Examples` on every public type, documenting replay vs execution semantics where relevant
  - [x] 1.5: Add unit tests verifying serde round-trip for all types

- [x] Task 2: Implement `DurableError` enum in `error.rs` (AC: #2, #3, #4)
  - [x] 2.1: Define `DurableError` enum with `#[derive(Debug, thiserror::Error)]` and these variants:
    - `ReplayMismatch` — expected vs actual operation info + cursor position
    - `CheckpointFailed` — operation name + source error
    - `Serialization` — type name + source serde_json error
    - `Deserialization` — type name + source serde_json error
    - `AwsSdk` — wrapping `aws_sdk_lambda::Error` with `#[from]`
    - `AwsSdkOperation` — wrapping specific operation errors (boxed) for individual API call failures
  - [x] 2.2: Make all variant fields private (use `struct` variants or tuple with private inner) and provide constructor methods: `replay_mismatch(expected, got, position)`, `checkpoint_failed(operation_name, source)`, `serialization(type_name, source)`, `deserialization(type_name, source)`
  - [x] 2.3: Implement `#[error("...")]` display messages with meaningful context for each variant
  - [x] 2.4: Add rustdoc with `# Examples` and `# Errors` sections on the enum and each constructor method
  - [x] 2.5: Add unit tests for error construction, display messages, and error chain propagation (`source()`)

- [x] Task 3: Update `lib.rs` re-exports (AC: #1, #2)
  - [x] 3.1: Add `pub use` statements in `lib.rs` to re-export all public types from `types` and `error` modules

- [x] Task 4: Verify all checks pass (AC: #5)
  - [x] 4.1: Run `cargo test --doc -p durable-lambda-core` — all doc examples compile and pass
  - [x] 4.2: Run `cargo test -p durable-lambda-core` — all unit tests pass
  - [x] 4.3: Run `cargo clippy -p durable-lambda-core -- -D warnings` — no warnings
  - [x] 4.4: Run `cargo fmt --check` — formatting passes
  - [x] 4.5: Run `cargo build --workspace` — full workspace still builds

## Dev Notes

### Critical Architecture Constraints

- **Constructor methods only**: All `DurableError` variants MUST be constructed via `DurableError::variant_name(args)` static methods. Users should NEVER construct variants directly via struct syntax. Keep variant inner fields private.
- **serde bounds**: `HistoryEntry` stores serialized values as `serde_json::Value` — this avoids generic type parameters on the struct itself while allowing any serializable type to be stored/retrieved.
- **CheckpointResult vs std::result::Result**: `CheckpointResult<T, E>` is a separate enum from `Result` — it represents a checkpointed step outcome (both success and error are valid, serialized values). Do NOT alias `std::result::Result`.
- **ExecutionMode simplicity**: `ExecutionMode` should NOT carry the history/cursor data. It's a simple discriminant. The replay engine (`replay.rs`, Story 1.3) owns the `Vec<HistoryEntry>` and cursor `usize`. `ExecutionMode` is just used to signal which mode the engine is in.
- **No BatchResult yet**: `BatchResult<T>` is mentioned in the PRD types list but it belongs to parallel/map operations (Epic 3). Do NOT implement it in this story.
- **lib.rs = re-exports only**: `lib.rs` must contain only `pub mod` and `pub use` statements. Zero logic.

### DurableError Variant Design

```rust
// Target error enum shape (fields are private — exposed via constructors):
#[derive(Debug, thiserror::Error)]
pub enum DurableError {
    #[error("replay mismatch at position {position}: expected {expected}, got {actual}")]
    ReplayMismatch {
        expected: String,
        actual: String,
        position: usize,
    },

    #[error("checkpoint failed for operation '{operation_name}': {source}")]
    CheckpointFailed {
        operation_name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to serialize type '{type_name}': {source}")]
    Serialization {
        type_name: String,
        source: serde_json::Error,
    },

    #[error("failed to deserialize type '{type_name}': {source}")]
    Deserialization {
        type_name: String,
        source: serde_json::Error,
    },

    #[error("AWS SDK error: {0}")]
    AwsSdk(#[from] aws_sdk_lambda::Error),

    #[error("AWS operation error: {0}")]
    AwsSdkOperation(Box<dyn std::error::Error + Send + Sync>),
}
```

**Important**: The `CheckpointFailed` variant uses `Box<dyn Error + Send + Sync>` for the source because checkpoint failures can come from different underlying sources (AWS errors, I/O errors, etc.). The `AwsSdkOperation` variant is needed because individual AWS operation errors (e.g., `SdkError<GetDurableExecutionHistoryError>`) are different from the general `aws_sdk_lambda::Error` type.

### Constructor Methods Pattern

```rust
impl DurableError {
    /// Create a replay mismatch error.
    pub fn replay_mismatch(expected: impl Into<String>, actual: impl Into<String>, position: usize) -> Self {
        Self::ReplayMismatch {
            expected: expected.into(),
            actual: actual.into(),
            position,
        }
    }

    /// Create a checkpoint failure error.
    pub fn checkpoint_failed(operation_name: impl Into<String>, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::CheckpointFailed {
            operation_name: operation_name.into(),
            source: Box::new(source),
        }
    }

    // ... etc for serialization, deserialization
}
```

### HistoryEntry Design

```rust
/// A single entry from the durable execution history log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The name/key identifying this operation (e.g., step name).
    pub name: String,
    /// The serialized result stored as a JSON value.
    pub result: serde_json::Value,
    /// The type of durable operation that produced this entry.
    pub operation_type: OperationType,
}

/// The type of durable operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationType {
    Step,
    Wait,
    Callback,
    Invoke,
    Parallel,
    Map,
    ChildContext,
    Log,
}
```

**Note**: The exact field names and structure for `HistoryEntry` may need to match the Python SDK's checkpoint format. During implementation, study the Python SDK's `_api/_history.py` or similar to determine the actual field names used in AWS. If unsure, use the names above as reasonable defaults — they can be adjusted in Story 1.3 when the replay engine is built.

### Rustdoc Convention

Every public item must have:
- Summary line in imperative mood ("Represent a checkpoint record", not "Represents")
- Document replay vs execution semantics where relevant
- `# Examples` section with compilable example
- `# Errors` section on anything returning `Result`

### thiserror 2.x Notes

Using thiserror `2.0.18`:
- `#[derive(thiserror::Error)]` works the same as 1.x
- `#[error("...")]` format syntax unchanged
- `#[from]` for auto `From` implementations works
- `#[source]` for error chain propagation works
- No separate `thiserror-impl` crate needed

### Testing Approach

- Unit tests in `#[cfg(test)] mod tests` at bottom of each file
- Test serde round-trip: serialize then deserialize, assert equality
- Test error display messages contain expected context
- Test error `source()` chain for wrapped errors
- Test `CheckpointResult` serialization for both `Ok` and `Err` variants
- Doc tests must compile standalone (include necessary `use` statements)

### Project Structure Notes

Files to modify/create:
```
crates/durable-lambda-core/src/
  lib.rs       — ADD pub use re-exports for types and error
  types.rs     — REPLACE stub with full type implementations
  error.rs     — REPLACE stub with full DurableError implementation
```

No new files needed. No other crates need changes.

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Error Strategy]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.2]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements - Error Handling (FR48-FR50)]
- [Source: _bmad-output/planning-artifacts/prd.md#API Surface - Core Types]
- [Source: _bmad-output/implementation-artifacts/1-1-project-workspace-initialization.md — workspace structure, dependency versions]

### Previous Story Intelligence (Story 1.1)

- Workspace structure confirmed working: virtual manifest with `[workspace.dependencies]`
- All 6 crates compile with current stub files
- `types.rs` and `error.rs` are currently doc-comment-only stubs ready to be replaced
- `lib.rs` currently has `pub mod` only — needs `pub use` additions
- CI pipeline (GitHub Actions) runs fmt, clippy, build, test
- thiserror `2.0.18` is pinned in workspace dependencies
- serde `1.0.228` with `derive` feature is pinned
- aws-sdk-lambda `1.118.0` is pinned
- Review follow-ups from 1.1: crate-level doc comments missing on approach crates (not relevant to this story)

## Dev Agent Record

### Agent Model Used

Claude Opus 4.6

### Debug Log References

### Completion Notes List

- Implemented `HistoryEntry` struct with `name`, `result` (serde_json::Value), and `operation_type` fields — all serde-serializable
- Implemented `OperationType` enum with all 8 durable operation variants
- Implemented `ExecutionMode` enum as simple discriminant (Replaying/Executing) with Serialize + Deserialize — no data fields per architecture
- Implemented `CheckpointResult<T, E>` as separate enum from `std::result::Result` with Ok/Err variants, serde-serializable
- Implemented `DurableError` enum with 6 variants: ReplayMismatch, CheckpointFailed, Serialization, Deserialization, AwsSdk (#[from]), AwsSdkOperation
- All DurableError variants constructed via static methods only — replay_mismatch(), checkpoint_failed(), serialization(), deserialization(), aws_sdk_operation()
- Constructor methods accept `impl Into<String>` for ergonomic API
- Full rustdoc with `# Examples` on every public type and constructor method
- DurableError is Send + Sync (verified by test)
- 16 unit tests: 8 for types (serde round-trip, equality), 8 for errors (display, source chain, From impl, Send+Sync)
- 10 doc tests all compile and pass
- Zero regressions across full workspace (all 6 crates build and test)
- Re-exports added to lib.rs: DurableError, CheckpointResult, ExecutionMode, HistoryEntry, OperationType
- [Code Review Fix] Added Serialize + Deserialize derives to ExecutionMode (AC #1 compliance)
- [Code Review Fix] Added #[non_exhaustive] to DurableError enum and all struct variants (enforces constructor-only pattern)

### Senior Developer Review (AI)

**Review Date:** 2026-03-14
**Outcome:** Approve (after fixes)

**Findings Fixed:**
- [x] [HIGH] ExecutionMode missing Serialize + Deserialize — added derives + serde round-trip test
- [x] [Med] DurableError struct variant fields publicly constructable — added #[non_exhaustive] on struct variants
- [x] [Low] DurableError enum not #[non_exhaustive] — added for forward compatibility

### File List

- crates/durable-lambda-core/src/types.rs (modified — replaced stub with full implementations)
- crates/durable-lambda-core/src/error.rs (modified — replaced stub with full DurableError implementation)
- crates/durable-lambda-core/src/lib.rs (modified — added pub use re-exports)

### Change Log

- 2026-03-14: Story 1.2 implemented — core types (HistoryEntry, OperationType, ExecutionMode, CheckpointResult) and DurableError enum with constructor methods, 25 tests passing
