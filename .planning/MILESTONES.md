# Milestones

## v1.2 Crates.io Publishing (Shipped: 2026-03-19)

**Phases:** 18-20 (3 phases, 6 plans)

**Key accomplishments:**
- All 6 SDK crates published to crates.io v1.2.0 (core, macro, closure, trait, builder, testing)
- Workspace-level version inheritance with dual MIT/Apache-2.0 license
- Standalone comprehensive README.md for each crate (195-299 lines)
- Dependency-ordered publish script with dry-run validation and idempotent re-runs
- GitHub Actions release workflow — v* tag triggers test suite, publish, GitHub Release
- PR publish-check job catches metadata issues before merging

---

## v1.1 AWS Integration Testing (Shipped: 2026-03-18)

**Phases:** 10-17 (8 phases, 12 plans)

**Key accomplishments:**
- Terraform infrastructure for 48 Lambda functions with durable execution
- Docker build pipeline with cargo-chef caching and musl cross-compilation
- Automated test harness — 48/48 tests passing (32 real + 16 XFAIL for unsupported service ops)
- Documented AWS durable execution service limitations (Context ops, ChainedInvoke, Callbacks)
- AWS CLI upgraded to 2.34.12 for durable execution command support

---

## v1.0 Production Hardening (Shipped: 2026-03-17)

**Phases:** 1-9 (9 phases, 23 plans)

**Key accomplishments:**
- Comprehensive error path tests (11 tests) and step closure panic safety
- Boundary condition tests (19 tests: nesting, replay determinism, edge values)
- DurableContextOps shared trait eliminating ~1,800 lines of delegation duplication
- Input validation guards + structured error codes on DurableError
- Step timeout, conditional retry, saga/compensation pattern, batch checkpoint
- Proc-macro type validation + builder .with_tracing()/.with_error_handler()
- Documentation overhaul (determinism rules, error examples, troubleshooting FAQ)

---

