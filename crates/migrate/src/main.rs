use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};

static PLATFORM_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/platform");

static TENANT_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/tenant");

#[tokio::main]
async fn main() -> Result<()> {
    let platform_url =
        std::env::var("PLATFORM_DATABASE_URL").context("PLATFORM_DATABASE_URL must be set")?;
    let tenant_url =
        std::env::var("TENANT_DATABASE_URL").context("TENANT_DATABASE_URL must be set")?;

    // ----------------------------------------------------------------
    // 1. Platform migrations (agencies, users, subscriptions, plans…)
    // ----------------------------------------------------------------
    println!("→ Connecting to platform database…");
    let platform_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&platform_url)
        .await
        .context("Failed to connect to platform database")?;

    println!("→ Running platform migrations…");
    PLATFORM_MIGRATOR
        .run(&platform_pool)
        .await
        .context("Platform migrations failed")?;
    println!("✓ Platform migrations complete");

    // ----------------------------------------------------------------
    // 2. Tenant migrations — one schema per agency, on the tenant DB
    // ----------------------------------------------------------------
    println!("→ Connecting to tenant database…");
    let tenant_base_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&tenant_url)
        .await
        .context("Failed to connect to tenant database")?;

    // Fetch all agency schemas from the platform DB
    let agencies: Vec<(String,)> =
        sqlx::query_as("SELECT schema_name FROM agencies ORDER BY created_at")
            .fetch_all(&platform_pool)
            .await
            .context("Failed to fetch agencies")?;

    if agencies.is_empty() {
        println!("→ No agencies found — seeding dev tenant schema for sqlx prepare…");
        seed_dev_agency(
            &platform_pool,
            &platform_url,
            &tenant_url,
            &tenant_base_pool,
        )
        .await?;
    } else {
        for (schema_name,) in &agencies {
            migrate_tenant_schema(&tenant_url, schema_name).await?;
        }
    }

    println!("✓ All tenant migrations complete");

    platform_pool.close().await;
    tenant_base_pool.close().await;
    Ok(())
}

/// Runs tenant migrations for a single schema on the tenant database.
async fn migrate_tenant_schema(tenant_url: &str, schema_name: &str) -> Result<()> {
    println!("  → Migrating tenant schema '{schema_name}'…");

    let schema = schema_name.to_owned();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |conn, _| {
            let s = schema.clone();
            Box::pin(async move {
                conn.execute(format!("SET search_path TO {s}, public").as_str())
                    .await?;
                Ok(())
            })
        })
        .connect(tenant_url)
        .await
        .with_context(|| format!("Failed to connect for schema '{schema_name}'"))?;

    pool.execute(format!("CREATE SCHEMA IF NOT EXISTS {schema_name}").as_str())
        .await?;

    TENANT_MIGRATOR
        .run(&pool)
        .await
        .with_context(|| format!("Tenant migrations failed for schema '{schema_name}'"))?;

    pool.close().await;
    println!("  ✓ Schema '{schema_name}' done");
    Ok(())
}

async fn seed_dev_agency(
    platform_pool: &PgPool,
    platform_url: &str, // ← add this parameter
    tenant_url: &str,
    tenant_pool: &PgPool,
) -> Result<()> {
    let dev_schema =
        std::env::var("DEV_TENANT_SCHEMA").unwrap_or_else(|_| "dev_agency".to_string());

    // Register placeholder row on platform DB
    sqlx::query(
        r#"
        INSERT INTO agencies
            (id, name, slug, schema_name, country_code, currency_code)
        VALUES
            (gen_random_uuid(), 'Dev Agency', $1, $1, 'KE', 'KES')
        ON CONFLICT (slug) DO NOTHING
        "#,
    )
    .bind(&dev_schema)
    .execute(platform_pool)
    .await
    .context("Failed to seed dev agency")?;

    // Migrate tenant schema on the TENANT database (for runtime)
    tenant_pool
        .execute(format!("CREATE SCHEMA IF NOT EXISTS {dev_schema}").as_str())
        .await?;
    migrate_tenant_schema(tenant_url, &dev_schema).await?;

    // ALSO migrate tenant schema on the PLATFORM database (for cargo sqlx prepare)
    // This lets a single DATABASE_URL see all tables via search_path.
    println!("  → Seeding tenant schema '{dev_schema}' on platform DB for sqlx prepare…");
    migrate_tenant_schema(platform_url, &dev_schema).await?;

    println!("  ✓ Dev tenant schema '{dev_schema}' seeded on both databases");
    Ok(())
}
