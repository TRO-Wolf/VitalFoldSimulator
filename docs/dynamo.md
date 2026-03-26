```JSON
{
    "tables": [
        {
            "table_name": "patient_visit",
            "spec": {
                "patient_id": "string",
                "clinic_id": "string",
                "provider_id": "string",
                "checkin_time": "string",
                "checkout_time": "string",
                "provider_seen_time": "string",
                "ekg_usage":"bool",
                "estimated_copay":"Decimal",
                "creation_time":"number",
                "record_expiration_epoch":"number",
                "height":"Decimal",
                "weight":"Decimal",
                "blood_pressure":"string",
                "heart_rate":"Decimal",
                "temperature":"Decimal",
                "oxygen_saturation":"Decimal",
                "pulse_rate":"Decimal"
            },
            "partition_key": "patient_id",
            "sort_key": "clinic_id"
        }
    ]
}
```