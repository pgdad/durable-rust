#!/usr/bin/env bash
# scripts/publish.sh
# Publishes all 6 durable-lambda crates to crates.io in dependency order.
# Core and macro go first (no inter-crate runtime deps), then the four
# wrapper crates that depend on core.
#
# Usage:
#   scripts/publish.sh              # Live publish to crates.io
#   scripts/publish.sh --dry-run    # Validate packaging without publishing
#   scripts/publish.sh --help       # Show this usage info
#
# Prerequisites:
#   - cargo login (live publish only; dry-run needs no token)
#   - All changes committed (live publish requires clean working tree)
#
# Behavior:
#   - Dry-run:  validates packaging for all 6 crates:
#               * Full `cargo publish --dry-run` for independent crates (core, macro)
#               * `cargo package --list` + metadata checks for dependent crates
#                 (full publish --dry-run requires core to be on crates.io first)
#   - Live:     checks crates.io for already-published versions, skips them,
#               waits 30 s between publishes for crates.io indexing
#   - Aborts immediately on first failure (set -e)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ---------------------------------------------------------------------------
# Color helpers (auto-detect terminal vs pipe)
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    RED='\033[0;31m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    GREEN='' YELLOW='' RED='' BOLD='' RESET=''
fi

# ---------------------------------------------------------------------------
# Usage / help
# ---------------------------------------------------------------------------
usage() {
    echo "Usage: $0 [--dry-run | --help]"
    echo ""
    echo "Publish all 6 durable-lambda crates to crates.io in dependency order."
    echo ""
    echo "Options:"
    echo "  --dry-run   Validate packaging without publishing (no crates.io token needed)"
    echo "  --help      Show this help message"
    echo ""
    echo "Crate publish order:"
    echo "  1. durable-lambda-core      (no inter-crate runtime deps)"
    echo "  2. durable-lambda-macro     (no inter-crate runtime deps)"
    echo "  3. durable-lambda-closure   (depends on core)"
    echo "  4. durable-lambda-trait     (depends on core)"
    echo "  5. durable-lambda-builder   (depends on core)"
    echo "  6. durable-lambda-testing   (depends on core)"
}

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
DRY_RUN=false

case "${1:-}" in
    --dry-run)
        DRY_RUN=true
        ;;
    --help|-h)
        usage
        exit 0
        ;;
    "")
        # Live publish mode — no flag needed
        ;;
    *)
        echo -e "${RED}Error: unknown argument '$1'${RESET}" >&2
        usage >&2
        exit 1
        ;;
esac

# ---------------------------------------------------------------------------
# Crate publish order
# ---------------------------------------------------------------------------
# Wave 1: no inter-crate runtime deps
# Wave 2: depend on core (dev-dependencies are ignored by cargo publish)
CRATES=(
    durable-lambda-core
    durable-lambda-macro
    durable-lambda-closure
    durable-lambda-trait
    durable-lambda-builder
    durable-lambda-testing
)

# Wave 1 crates have no inter-workspace runtime deps and support full
# cargo publish --dry-run before anything is on crates.io
INDEPENDENT_CRATES=(
    durable-lambda-core
    durable-lambda-macro
)

# ---------------------------------------------------------------------------
# Extract workspace version from root Cargo.toml
# ---------------------------------------------------------------------------
VERSION=$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')
if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: could not extract version from $REPO_ROOT/Cargo.toml${RESET}" >&2
    exit 1
fi

echo -e "${BOLD}Publishing durable-lambda crates v${VERSION}${RESET}"
if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}Mode: dry-run (validation only, no crates.io token required)${RESET}"
else
    echo -e "${YELLOW}Mode: live publish to crates.io${RESET}"
fi
echo ""

# ---------------------------------------------------------------------------
# Already-published detection (live mode only)
# ---------------------------------------------------------------------------
is_published() {
    local crate_name="$1"
    local version="$2"
    local http_code

    http_code=$(curl -s -o /dev/null -w "%{http_code}" \
        "https://crates.io/api/v1/crates/${crate_name}/${version}")

    if [ "$http_code" = "200" ]; then
        return 0  # already published
    else
        return 1  # not published
    fi
}

# ---------------------------------------------------------------------------
# Check if a crate is in the independent set (no workspace runtime deps)
# ---------------------------------------------------------------------------
is_independent() {
    local crate="$1"
    for ind in "${INDEPENDENT_CRATES[@]}"; do
        if [ "$ind" = "$crate" ]; then
            return 0
        fi
    done
    return 1
}

# ---------------------------------------------------------------------------
# Dry-run validation for a single crate
# ---------------------------------------------------------------------------
dry_run_crate() {
    local crate="$1"
    local crate_dir="$2"

    cd "$crate_dir"

    if is_independent "$crate"; then
        # Independent crates: full cargo publish --dry-run
        echo "  Running: cargo publish --dry-run --allow-dirty"
        cargo publish --dry-run --allow-dirty
    else
        # Dependent crates: cargo package --list validates file set and metadata
        # Full publish --dry-run is not possible until core is on crates.io
        echo "  Validating: cargo package --list (full dry-run requires core on crates.io)"
        local file_list
        file_list=$(cargo package --list --allow-dirty 2>&1)
        local file_count
        file_count=$(echo "$file_list" | wc -l)

        # Verify essential files are included
        local has_cargo_toml=false
        local has_readme=false
        local has_src=false

        if echo "$file_list" | grep -q "^Cargo.toml$"; then has_cargo_toml=true; fi
        if echo "$file_list" | grep -q "^README.md$"; then has_readme=true; fi
        if echo "$file_list" | grep -q "^src/"; then has_src=true; fi

        echo "  Files in package: $file_count"
        if [ "$has_cargo_toml" = false ]; then
            echo -e "  ${RED}FAIL: Cargo.toml missing from package${RESET}" >&2
            return 1
        fi
        if [ "$has_readme" = false ]; then
            echo -e "  ${RED}FAIL: README.md missing from package${RESET}" >&2
            return 1
        fi
        if [ "$has_src" = false ]; then
            echo -e "  ${RED}FAIL: no src/ files in package${RESET}" >&2
            return 1
        fi

        # Verify version field exists in dependency declaration
        if grep -q 'durable-lambda-core.*version' "$crate_dir/Cargo.toml"; then
            echo "  Dependency version: declared (will resolve from crates.io on publish)"
        else
            echo -e "  ${RED}FAIL: durable-lambda-core dependency missing version field${RESET}" >&2
            return 1
        fi
    fi

    return 0
}

# ---------------------------------------------------------------------------
# Publish loop
# ---------------------------------------------------------------------------
PASSED=0
SKIPPED=0
TOTAL=${#CRATES[@]}

for i in "${!CRATES[@]}"; do
    CRATE="${CRATES[$i]}"
    INDEX=$((i + 1))
    CRATE_DIR="$REPO_ROOT/crates/$CRATE"

    echo -e "${BOLD}=== [$INDEX/$TOTAL] $CRATE ===${RESET}"

    if [ ! -d "$CRATE_DIR" ]; then
        echo -e "${RED}Error: crate directory not found: $CRATE_DIR${RESET}" >&2
        exit 1
    fi

    if [ "$DRY_RUN" = true ]; then
        dry_run_crate "$CRATE" "$CRATE_DIR"
        echo -e "  ${GREEN}PASS${RESET}"
        PASSED=$((PASSED + 1))
    else
        cd "$CRATE_DIR"

        # Live publish mode: check if already published, then publish
        if is_published "$CRATE" "$VERSION"; then
            echo -e "  ${YELLOW}SKIP: $CRATE v$VERSION already published on crates.io${RESET}"
            SKIPPED=$((SKIPPED + 1))
        else
            echo "  Running: cargo publish"
            local publish_output
            if publish_output=$(cargo publish 2>&1); then
                echo -e "  ${GREEN}PUBLISHED${RESET}"
                PASSED=$((PASSED + 1))
            elif echo "$publish_output" | grep -q "already exists"; then
                echo -e "  ${YELLOW}SKIP: $CRATE v$VERSION already exists on crates.io index${RESET}"
                SKIPPED=$((SKIPPED + 1))
                continue
            else
                echo "$publish_output" >&2
                echo -e "  ${RED}FAILED${RESET}" >&2
                exit 1
            fi

            # Wait for crates.io indexing before publishing dependents
            if [ "$INDEX" -lt "$TOTAL" ]; then
                echo -n "  Waiting for crates.io indexing: "
                for countdown in $(seq 30 -1 1); do
                    echo -n "${countdown}s "
                    sleep 1
                done
                echo "done"
            fi
        fi
    fi

    echo ""
done

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo -e "${BOLD}=== Summary ===${RESET}"
if [ "$DRY_RUN" = true ]; then
    echo -e "  Validated: ${GREEN}${PASSED}/${TOTAL}${RESET} crates"
    if [ "$PASSED" -eq "$TOTAL" ]; then
        echo -e "  ${GREEN}All crates passed dry-run validation.${RESET}"
    else
        echo -e "  ${RED}Some crates failed validation.${RESET}"
        exit 1
    fi
else
    echo -e "  Published: ${GREEN}${PASSED}${RESET}"
    echo -e "  Skipped:   ${YELLOW}${SKIPPED}${RESET}"
    PROCESSED=$((PASSED + SKIPPED))
    if [ "$PROCESSED" -eq "$TOTAL" ]; then
        echo -e "  ${GREEN}All $TOTAL crates processed successfully.${RESET}"
    else
        echo -e "  ${RED}Some crates failed to publish.${RESET}"
        exit 1
    fi
fi
