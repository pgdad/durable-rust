# Requirements: durable-rust

**Defined:** 2026-03-16
**Core Value:** Every durable operation behaves identically to the Python SDK — zero behavioral divergence

## v1.1 Requirements

Requirements for GSD Tooling Transition milestone. Each maps to roadmap phases.

### GSD Infrastructure

- [x] **GSD-01**: MILESTONES.md exists capturing v1.0 as shipped milestone with validated capabilities
- [x] **GSD-02**: REQUIREMENTS.md exists with REQ-IDs for all v1.1 scope items
- [x] **GSD-03**: ROADMAP.md exists with phased execution plan continuing from phase 1

### BMAD Cleanup

- [ ] **BMAD-01**: `_bmad-output/` directory removed from repository in a dedicated commit
- [ ] **BMAD-02**: `_bmad/` directory removed from repository in a dedicated commit
- [ ] **BMAD-03**: No orphaned references to `_bmad` remain in tracked files

## Future Requirements

Deferred to post-transition milestone. Tracked but not in current roadmap.

### SDK Features

- **SDK-01**: Publish crates to crates.io
- **SDK-02**: CI/CD pipeline with automated testing
- **SDK-03**: Performance benchmarks vs Python SDK

## Out of Scope

| Feature | Reason |
|---------|--------|
| Rust source code changes | No SDK changes in tooling transition milestone |
| CI/CD pipeline setup | Separate concern from tooling transition |
| Porting BMAD brainstorming content | Already captured in git history, v1.0 work is complete |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| GSD-01 | Phase 1 | Complete |
| GSD-02 | Phase 1 | Complete |
| GSD-03 | Phase 1 | Complete |
| BMAD-01 | Phase 2 | Pending |
| BMAD-02 | Phase 2 | Pending |
| BMAD-03 | Phase 2 | Pending |

**Coverage:**
- v1.1 requirements: 6 total
- Mapped to phases: 6
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-16*
*Last updated: 2026-03-16 after roadmap creation*
