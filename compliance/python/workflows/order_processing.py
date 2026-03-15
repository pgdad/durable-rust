"""
Order Processing Compliance Workflow (Python reference implementation).

Multi-step workflow: validate order → charge payment (with retries) → send confirmation.
Outputs the expected operation sequence as JSON for Rust compliance verification.

Usage:
    python order_processing.py > ../tests/fixtures/order_processing.json
"""

import json


def generate_expected_operations():
    """Generate the operation sequence that this workflow produces.

    The Python durable Lambda SDK records each operation as:
    - type: "step" for ctx.step() calls
    - name: the operation name string

    This workflow performs three sequential steps:
    1. validate_order — validates the incoming order
    2. charge_payment — charges payment (configured with retries)
    3. send_confirmation — sends order confirmation
    """
    return {
        "workflow": "order_processing",
        "operations": [
            {"type": "step", "name": "validate_order"},
            {"type": "step", "name": "charge_payment"},
            {"type": "step", "name": "send_confirmation"},
        ],
    }


if __name__ == "__main__":
    print(json.dumps(generate_expected_operations(), indent=2))
