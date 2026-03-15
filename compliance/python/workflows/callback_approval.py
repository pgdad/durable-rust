"""
Callback Approval Compliance Workflow (Python reference implementation).

Callback-based approval workflow: submit request, create callback for
external approval, wait for timeout, check approval result, finalize.

Usage:
    python callback_approval.py > ../tests/fixtures/callback_approval.json
"""

import json


def generate_expected_operations():
    """Generate the operation sequence that this workflow produces.

    The Python durable Lambda SDK records each operation as:
    - type: "step" for ctx.step() calls
    - type: "callback" for ctx.create_callback() calls
    - type: "wait" for ctx.wait() calls

    This workflow performs:
    1. submit_request — step: prepares and submits the approval request
    2. approval — callback: creates callback and waits for external signal
    3. approval_timeout — wait: waits for a timeout period
    4. process_approval — step: processes the approval decision
    5. finalize — step: finalizes the workflow
    """
    return {
        "workflow": "callback_approval",
        "operations": [
            {"type": "step", "name": "submit_request"},
            {"type": "callback", "name": "approval"},
            {"type": "wait", "name": "approval_timeout"},
            {"type": "step", "name": "process_approval"},
            {"type": "step", "name": "finalize"},
        ],
    }


if __name__ == "__main__":
    print(json.dumps(generate_expected_operations(), indent=2))
