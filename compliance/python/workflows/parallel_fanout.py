"""
Parallel Fanout Compliance Workflow (Python reference implementation).

Parallel branch workflow: validates input, then fans out to three parallel
processing branches (enrich, score, tag), then aggregates results.

Usage:
    python parallel_fanout.py > ../tests/fixtures/parallel_fanout.json
"""

import json


def generate_expected_operations():
    """Generate the operation sequence that this workflow produces.

    The Python durable Lambda SDK records each operation as:
    - type: "step" for ctx.step() calls

    This workflow performs:
    1. validate_input — validates the incoming data
    2. enrich_data — parallel branch 1: data enrichment
    3. score_data — parallel branch 2: data scoring
    4. tag_data — parallel branch 3: data tagging
    5. aggregate_results — combines parallel branch outputs

    Note: In the Python SDK, parallel branches are implemented as
    individual steps. The Rust SDK's parallel() operation records each
    branch as a separate step operation. Both produce the same sequence.
    """
    return {
        "workflow": "parallel_fanout",
        "operations": [
            {"type": "step", "name": "validate_input"},
            {"type": "step", "name": "enrich_data"},
            {"type": "step", "name": "score_data"},
            {"type": "step", "name": "tag_data"},
            {"type": "step", "name": "aggregate_results"},
        ],
    }


if __name__ == "__main__":
    print(json.dumps(generate_expected_operations(), indent=2))
