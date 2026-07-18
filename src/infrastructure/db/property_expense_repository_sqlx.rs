use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::property_expense_repository::PropertyExpenseRepository},
    domain::property_expense::{
        CreatePropertyExpenseCommand, PropertyExpense, PropertyExpenseCategory,
    },
};

pub struct PgPropertyExpenseRepo {
    pool: PgPool,
}

impl PgPropertyExpenseRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgPropertyExpenseRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PropertyExpenseRow {
    id: Uuid,
    agency_id: Uuid,
    property_id: Uuid,
    expense_date: time::Date,
    category: String,
    description: String,
    vendor_name: Option<String>,
    amount_kes: rust_decimal::Decimal,
    notes: Option<String>,
    created_by: Uuid,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

impl TryFrom<PropertyExpenseRow> for PropertyExpense {
    type Error = AppError;

    fn try_from(row: PropertyExpenseRow) -> Result<Self, Self::Error> {
        let category = PropertyExpenseCategory::from_str(&row.category)
            .ok_or_else(|| AppError::InternalServer(format!("unknown expense category: {}", row.category)))?;

        Ok(Self {
            id: row.id,
            agency_id: row.agency_id,
            property_id: row.property_id,
            expense_date: row.expense_date,
            category,
            description: row.description,
            vendor_name: row.vendor_name,
            amount_kes: row.amount_kes,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[async_trait]
impl PropertyExpenseRepository for PgPropertyExpenseRepo {
    async fn list_for_property(
        &self,
        agency_id: Uuid,
        property_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PropertyExpense>, AppError> {
        let rows = sqlx::query_as::<_, PropertyExpenseRow>(
            r#"
            SELECT
                id,
                agency_id,
                property_id,
                expense_date,
                category,
                description,
                vendor_name,
                amount_kes,
                notes,
                created_by,
                created_at,
                updated_at
            FROM property_expenses
            WHERE agency_id = $1
              AND property_id = $2
            ORDER BY expense_date DESC, created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(agency_id)
        .bind(property_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        rows.into_iter()
            .map(PropertyExpense::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn create(
        &self,
        command: CreatePropertyExpenseCommand,
    ) -> Result<PropertyExpense, AppError> {
        let row = sqlx::query_as::<_, PropertyExpenseRow>(
            r#"
            INSERT INTO property_expenses (
                agency_id,
                property_id,
                expense_date,
                category,
                description,
                vendor_name,
                amount_kes,
                notes,
                created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id,
                agency_id,
                property_id,
                expense_date,
                category,
                description,
                vendor_name,
                amount_kes,
                notes,
                created_by,
                created_at,
                updated_at
            "#,
        )
        .bind(command.agency_id)
        .bind(command.property_id)
        .bind(command.expense_date)
        .bind(command.category.as_str())
        .bind(command.description)
        .bind(command.vendor_name)
        .bind(command.amount_kes)
        .bind(command.notes)
        .bind(command.created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        PropertyExpense::try_from(row)
    }
}
