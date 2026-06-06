#!/bin/bash
for tbl in portal_user_index user_preferences property_owners tax_obligations wht_certificates billing_periods billing_notifications property_features chart_of_accounts agency_integrations property_billing_settings feature_overrides notification_templates; do
    echo "Checking $tbl:"
    grep -irn -A 10 -B 2 -E "(CREATE TABLE\s+$tbl|UNIQUE.*\(.*|PRIMARY KEY\s*\(.*)" migrations/ | grep -iE "(CREATE TABLE|UNIQUE|PRIMARY KEY|CONSTRAINT)"
    echo "---"
done
