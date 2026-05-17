use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::document_repository::DocumentRepository},
    domain::document::{CreateDocumentCommand, Document, DocumentFilter, DocumentType},
};

pub struct PgDocumentRepo {
    pool: PgPool,
}

impl PgDocumentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Internal row ──────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct DocumentRow {
    id: Uuid,
    agency_id: Uuid,
    s3_key: String,
    file_name: String,
    mime_type: String,
    size_bytes: i64,
    document_type: String,
    title: Option<String>,
    notes: Option<String>,
    property_id: Option<Uuid>,
    unit_id: Option<Uuid>,
    resident_id: Option<Uuid>,
    agreement_id: Option<Uuid>,
    work_order_id: Option<Uuid>,
    uploaded_by: Uuid,
    created_at: time::OffsetDateTime,
}

impl TryFrom<DocumentRow> for Document {
    type Error = AppError;
    fn try_from(r: DocumentRow) -> Result<Self, Self::Error> {
        let document_type = DocumentType::from_str(&r.document_type).ok_or_else(|| {
            AppError::InternalServer(format!("unknown doc type: {}", r.document_type))
        })?;
        Ok(Document {
            id: r.id,
            agency_id: r.agency_id,
            s3_key: r.s3_key,
            file_name: r.file_name,
            mime_type: r.mime_type,
            size_bytes: r.size_bytes,
            document_type,
            title: r.title,
            notes: r.notes,
            property_id: r.property_id,
            unit_id: r.unit_id,
            resident_id: r.resident_id,
            agreement_id: r.agreement_id,
            work_order_id: r.work_order_id,
            uploaded_by: r.uploaded_by,
            created_at: r.created_at,
        })
    }
}

// ── Impl ──────────────────────────────────────────────────────────────────────

#[async_trait]
impl DocumentRepository for PgDocumentRepo {
    async fn find_all(&self, filter: DocumentFilter) -> Result<Vec<Document>, AppError> {
        let type_str = filter
            .document_type
            .as_ref()
            .map(|t| t.as_str().to_string());
        let rows = sqlx::query_as::<_, DocumentRow>(
            r#"
            SELECT id, agency_id, s3_key, file_name, mime_type, size_bytes,
                   document_type, title, notes,
                   property_id, unit_id, resident_id, agreement_id, work_order_id,
                   uploaded_by, created_at
            FROM   documents
            WHERE  agency_id    = $1
              AND ($2::uuid IS NULL OR property_id   = $2)
              AND ($3::uuid IS NULL OR unit_id        = $3)
              AND ($4::uuid IS NULL OR resident_id    = $4)
              AND ($5::uuid IS NULL OR agreement_id   = $5)
              AND ($6::uuid IS NULL OR work_order_id  = $6)
              AND ($7::text IS NULL OR document_type  = $7)
            ORDER BY created_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(filter.agency_id)
        .bind(filter.property_id)
        .bind(filter.unit_id)
        .bind(filter.resident_id)
        .bind(filter.agreement_id)
        .bind(filter.work_order_id)
        .bind(type_str)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        rows.into_iter().map(Document::try_from).collect()
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Document>, AppError> {
        let row = sqlx::query_as::<_, DocumentRow>(
            r#"
            SELECT id, agency_id, s3_key, file_name, mime_type, size_bytes,
                   document_type, title, notes,
                   property_id, unit_id, resident_id, agreement_id, work_order_id,
                   uploaded_by, created_at
            FROM   documents
            WHERE  id = $1 AND agency_id = $2
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        row.map(Document::try_from).transpose()
    }

    async fn create(&self, cmd: CreateDocumentCommand) -> Result<Document, AppError> {
        let row = sqlx::query_as::<_, DocumentRow>(
            r#"
            INSERT INTO documents (
                id, agency_id, s3_key, file_name, mime_type, size_bytes,
                document_type, title, notes,
                property_id, unit_id, resident_id, agreement_id, work_order_id,
                uploaded_by
            )
            VALUES (
                uuidv7(), $1, $2, $3, $4, $5,
                $6, $7, $8,
                $9, $10, $11, $12, $13,
                $14
            )
            RETURNING id, agency_id, s3_key, file_name, mime_type, size_bytes,
                      document_type, title, notes,
                      property_id, unit_id, resident_id, agreement_id, work_order_id,
                      uploaded_by, created_at
            "#,
        )
        .bind(cmd.agency_id)
        .bind(cmd.s3_key)
        .bind(cmd.file_name)
        .bind(cmd.mime_type)
        .bind(cmd.size_bytes)
        .bind(cmd.document_type.as_str())
        .bind(cmd.title)
        .bind(cmd.notes)
        .bind(cmd.property_id)
        .bind(cmd.unit_id)
        .bind(cmd.resident_id)
        .bind(cmd.agreement_id)
        .bind(cmd.work_order_id)
        .bind(cmd.uploaded_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Document::try_from(row)
    }

    /// Deletes the row and returns the S3 key so the caller can remove the file.
    async fn delete(&self, agency_id: Uuid, id: Uuid) -> Result<String, AppError> {
        let row =
            sqlx::query("DELETE FROM documents WHERE id = $1 AND agency_id = $2 RETURNING s3_key")
                .bind(id)
                .bind(agency_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?
                .ok_or_else(|| AppError::NotFound(format!("document {id}")))?;

        use sqlx::Row;
        Ok(row.get::<String, _>("s3_key"))
    }

    async fn count_for_agency(&self, agency_id: Uuid) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) FROM documents WHERE agency_id = $1")
            .bind(agency_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.get::<i64, _>(0))
    }
}
