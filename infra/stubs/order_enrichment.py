import json


def lambda_handler(event, context):
    order_id = event.get("order_id", "unknown")
    return {
        "enriched": True,
        "order_id": order_id,
        "details": {"priority": "standard", "region": "us-east-2"}
    }
