// src/infrastructure/db/property_billing_repository_sqlx.rs

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError, ports::property_billing_repository::PropertyBillingRepository,
    },
    domain::property_billing::{PropertyBillingSettings, PropertyBillingSummary},
};

pub struct PgPropertyBillingRepo {
    pool: PgPool,
}

impl PgPropertyBillingRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PropertyBillingRepository for PgPropertyBillingRepo {
    async fn get_by_property_id(
        &self,
        property_id: Uuid,
    ) -> Result<Option<PropertyBillingSettings>, AppError> {
        let row = sqlx::query_as::<_, PropertyBillingSettings>(
            r#"
            SELECT property_id, currency_code, rent_due_day, late_fee_type, late_fee_value, late_fee_grace_days,
                   water_rate_per_unit, garbage_fee_kes, security_fee_kes, other_fixed_fees
            FROM property_billing_settings
            WHERE property_id = $1
            "#,
        )
        .bind(property_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row)
    }

    async fn upsert(&self, s: PropertyBillingSettings) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO property_billing_settings (
                property_id, currency_code, rent_due_day, late_fee_type, late_fee_value, late_fee_grace_days,
                water_rate_per_unit, garbage_fee_kes, security_fee_kes, other_fixed_fees
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (property_id) DO UPDATE SET
                currency_code = EXCLUDED.currency_code,
                rent_due_day = EXCLUDED.rent_due_day,
                late_fee_type = EXCLUDED.late_fee_type,
                late_fee_value = EXCLUDED.late_fee_value,
                late_fee_grace_days = EXCLUDED.late_fee_grace_days,
                water_rate_per_unit = EXCLUDED.water_rate_per_unit,
                garbage_fee_kes = EXCLUDED.garbage_fee_kes,
                security_fee_kes = EXCLUDED.security_fee_kes,
                other_fixed_fees = EXCLUDED.other_fixed_fees,
                updated_at = now()
            "#,
        )
        .bind(s.property_id)
        .bind(s.currency_code)
        .bind(s.rent_due_day)
        .bind(s.late_fee_type)
        .bind(s.late_fee_value)
        .bind(s.late_fee_grace_days)
        .bind(s.water_rate_per_unit)
        .bind(s.garbage_fee_kes)
        .bind(s.security_fee_kes)
        .bind(s.other_fixed_fees)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }

    async fn get_agency_summary(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<PropertyBillingSummary>, AppError> {
        let rows = sqlx::query_as::<_, PropertyBillingSummary>(
            r#"
            SELECT p.id as property_id, p.name as property_name, 
                   s.currency_code, s.rent_due_day, 
                   s.late_fee_type, s.late_fee_value, s.late_fee_grace_days,
                   s.water_rate_per_unit, s.garbage_fee_kes, s.security_fee_kes
            FROM properties p
            JOIN property_billing_settings s ON s.property_id = p.id
            WHERE p.agency_id = $1
            ORDER BY p.name ASC
            "#,
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(rows)
    }
}
