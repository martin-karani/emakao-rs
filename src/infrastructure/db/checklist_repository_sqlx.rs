

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::checklist_repository::{
            ChecklistRepository, CreateChecklistCommand, CreateChecklistItemCommand,
            CreateChecklistSectionCommand, InstantiateChecklistTreeCommand, UpdateChecklistCommand,
        },
    },
    domain::checklist::{Checklist, ChecklistItem, ChecklistSection, ChecklistType},
};

#[derive(Debug, sqlx::FromRow)]
struct ChecklistRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    is_default: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<ChecklistRow> for Checklist {
    type Error = AppError;

    fn try_from(row: ChecklistRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            description: row.description,
            is_default: row.is_default,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ChecklistSectionRow {
    id: Uuid,
    checklist_id: Uuid,
    name: String,
    description: Option<String>,
    sort_order: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<ChecklistSectionRow> for ChecklistSection {
    type Error = AppError;

    fn try_from(row: ChecklistSectionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            checklist_id: row.checklist_id,
            name: row.name,
            description: row.description,
            sort_order: row.sort_order,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ChecklistItemRow {
    id: Uuid,
    section_id: Uuid,
    name: String,
    description: Option<String>,
    checklist_type: String,
    sort_order: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<ChecklistItemRow> for ChecklistItem {
    type Error = AppError;

    fn try_from(row: ChecklistItemRow) -> Result<Self, Self::Error> {
        let checklist_type = ChecklistType::from_str(&row.checklist_type)
            .map_err(|_| AppError::Validation("Invalid checklist type".to_string()))?;

        Ok(Self {
            id: row.id,
            section_id: row.section_id,
            name: row.name,
            description: row.description,
            checklist_type,
            sort_order: row.sort_order,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub struct PgChecklistRepo {
    pool: PgPool,
}

impl From<PgPool> for PgChecklistRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChecklistRepository for PgChecklistRepo {
    async fn create_checklist(&self, cmd: CreateChecklistCommand) -> Result<Checklist, AppError> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let row = sqlx::query(
            r#"
            INSERT INTO checklists (id, name, description, is_default, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, description, is_default, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(cmd.name)
        .bind(cmd.description)
        .bind(cmd.is_default)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        let checklist_row = ChecklistRow {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            is_default: row.try_get("is_default")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        checklist_row.try_into()
    }

    async fn get_checklist(&self, id: Uuid) -> Result<Checklist, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, is_default, created_at, updated_at
            FROM checklists
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let checklist_row = ChecklistRow {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            is_default: row.try_get("is_default")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        checklist_row.try_into()
    }

    async fn list_checklists(&self) -> Result<Vec<Checklist>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, is_default, created_at, updated_at
            FROM checklists
            WHERE id NOT IN (SELECT checklist_id FROM property_checklists)
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut checklists = Vec::with_capacity(rows.len());
        for row in rows {
            let checklist_row = ChecklistRow {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                is_default: row.try_get("is_default")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            checklists.push(checklist_row.try_into()?);
        }

        Ok(checklists)
    }

    async fn update_checklist(&self, cmd: UpdateChecklistCommand) -> Result<Checklist, AppError> {
        let row = sqlx::query(
            r#"
            UPDATE checklists
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                is_default = COALESCE($4, is_default)
            WHERE id = $1
            RETURNING id, name, description, is_default, created_at, updated_at
            "#,
        )
        .bind(cmd.id)
        .bind(cmd.name)
        .bind(cmd.description)
        .bind(cmd.is_default)
        .fetch_one(&self.pool)
        .await?;

        let checklist_row = ChecklistRow {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            is_default: row.try_get("is_default")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        checklist_row.try_into()
    }

    async fn delete_checklist(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM checklists
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_section(&self, cmd: CreateChecklistSectionCommand) -> Result<ChecklistSection, AppError> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let row = sqlx::query(
            r#"
            INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, checklist_id, name, description, sort_order, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(cmd.checklist_id)
        .bind(cmd.name)
        .bind(cmd.description)
        .bind(cmd.sort_order)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        let section_row = ChecklistSectionRow {
            id: row.try_get("id")?,
            checklist_id: row.try_get("checklist_id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            sort_order: row.try_get("sort_order")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        section_row.try_into()
    }

    async fn list_sections(&self, checklist_id: Uuid) -> Result<Vec<ChecklistSection>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT cs.id, cs.checklist_id, cs.name, cs.description, cs.sort_order, cs.created_at, cs.updated_at
            FROM checklist_sections cs
            WHERE cs.checklist_id = $1
            ORDER BY cs.sort_order ASC
            "#,
        )
        .bind(checklist_id)
        .fetch_all(&self.pool)
        .await?;

        let mut sections = Vec::with_capacity(rows.len());
        for row in rows {
            let section_row = ChecklistSectionRow {
                id: row.try_get("id")?,
                checklist_id: row.try_get("checklist_id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                sort_order: row.try_get("sort_order")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            sections.push(section_row.try_into()?);
        }

        Ok(sections)
    }

    async fn create_item(&self, cmd: CreateChecklistItemCommand) -> Result<ChecklistItem, AppError> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let checklist_type_str = cmd.checklist_type.to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, section_id, name, description, checklist_type, sort_order, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(cmd.section_id)
        .bind(cmd.name)
        .bind(cmd.description)
        .bind(checklist_type_str)
        .bind(cmd.sort_order)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        let item_row = ChecklistItemRow {
            id: row.try_get("id")?,
            section_id: row.try_get("section_id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            checklist_type: row.try_get("checklist_type")?,
            sort_order: row.try_get("sort_order")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        item_row.try_into()
    }

    async fn list_items(&self, section_id: Uuid) -> Result<Vec<ChecklistItem>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT ci.id, ci.section_id, ci.name, ci.description, ci.checklist_type, ci.sort_order, ci.created_at, ci.updated_at
            FROM checklist_items ci
            WHERE ci.section_id = $1
            ORDER BY ci.sort_order ASC
            "#,
        )
        .bind(section_id)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let item_row = ChecklistItemRow {
                id: row.try_get("id")?,
                section_id: row.try_get("section_id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                checklist_type: row.try_get("checklist_type")?,
                sort_order: row.try_get("sort_order")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            items.push(item_row.try_into()?);
        }

        Ok(items)
    }

    async fn attach_checklist_to_property(&self, property_id: Uuid, checklist_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO property_checklists (property_id, checklist_id)
            VALUES ($1, $2)
            ON CONFLICT (property_id, checklist_id) DO NOTHING
            "#,
        )
        .bind(property_id)
        .bind(checklist_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_property_checklists(&self, property_id: Uuid) -> Result<Vec<Checklist>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT c.id, c.name, c.description, c.is_default, c.created_at, c.updated_at
            FROM checklists c
            JOIN property_checklists pc ON c.id = pc.checklist_id
            WHERE pc.property_id = $1
            ORDER BY c.created_at DESC
            "#,
        )
        .bind(property_id)
        .fetch_all(&self.pool)
        .await?;

        let mut checklists = Vec::with_capacity(rows.len());
        for row in rows {
            let checklist_row = ChecklistRow {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                is_default: row.try_get("is_default")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            checklists.push(checklist_row.try_into()?);
        }

        Ok(checklists)
    }

    async fn detach_checklist_from_property(&self, property_id: Uuid, checklist_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM property_checklists
            WHERE property_id = $1 AND checklist_id = $2
            "#,
        )
        .bind(property_id)
        .bind(checklist_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn instantiate_tree(&self, cmd: InstantiateChecklistTreeCommand) -> Result<Checklist, AppError> {
        let checklist_id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let mut tx = self.pool.begin().await?;

        // Insert the checklist
        let row = sqlx::query(
            r#"
            INSERT INTO checklists (id, name, description, is_default, created_at, updated_at)
            VALUES ($1, $2, $3, false, $4, $5)
            RETURNING id, name, description, is_default, created_at, updated_at
            "#,
        )
        .bind(checklist_id)
        .bind(&cmd.name)
        .bind(&cmd.description)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        let checklist = Checklist {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            is_default: row.try_get("is_default")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        // Link checklist to property
        sqlx::query(
            r#"
            INSERT INTO property_checklists (property_id, checklist_id)
            VALUES ($1, $2)
            ON CONFLICT (property_id, checklist_id) DO NOTHING
            "#,
        )
        .bind(cmd.property_id)
        .bind(checklist_id)
        .execute(&mut *tx)
        .await?;

        // Insert sections and items
        for section in cmd.sections {
            let section_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(section_id)
            .bind(checklist_id)
            .bind(&section.name)
            .bind(&section.description)
            .bind(section.sort_order)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            for item in section.items {
                let item_id = Uuid::new_v4();
                let type_str = item.checklist_type.to_string();
                sqlx::query(
                    r#"
                    INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    "#,
                )
                .bind(item_id)
                .bind(section_id)
                .bind(&item.name)
                .bind(&item.description)
                .bind(type_str)
                .bind(item.sort_order)
                .bind(now)
                .bind(now)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        Ok(checklist)
    }
}
