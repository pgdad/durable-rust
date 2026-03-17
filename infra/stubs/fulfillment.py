import json


def lambda_handler(event, context):
    return {
        "fulfillment_id": "ff-001",
        "status": "started",
        "estimated_delivery": "2 business days"
    }
