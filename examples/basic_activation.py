"""
PyReverseETL Example: Basic Data Activation

This example shows how to:
1. Connect to a data warehouse
2. Define an activation target (CRM, email, etc.)
3. Activate customer segments
"""

from pyre import Warehouse, Activation, ActivationTarget

# Connect to warehouse
warehouse = Warehouse(
    connection_string="postgres://user:pass@localhost/warehouse"
)

# Define activation target
salesforce = ActivationTarget(
    provider="salesforce",
    credentials={"client_id": "...", "client_secret": "..."}
)

# Define activation: Send high-value customers to Salesforce
activation = Activation(
    name="high_value_customers",
    source_table="customers",
    filter="lifetime_value > 50000",
    target=salesforce,
    mapping={
        "customer_id": "external_id",
        "email": "email",
        "lifetime_value": "custom_field_revenue"
    }
)

# Execute activation
result = activation.execute()
print(f"Activated {result.records_processed} customers")
print(f"Success rate: {result.success_rate:.1%}")
