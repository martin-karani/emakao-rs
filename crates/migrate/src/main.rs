use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};

mod seed;

static PLATFORM_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/platform");
static AGENCY_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/agency");

// ── Config ────────────────────────────────────────────────────────────────────

struct Config {
    platform_url: String,
    agency_url: String,
    /// MIGRATE_RESET=true  → drop + recreate all schemas before migrating.
    /// Equivalent to wiping the DB without touching Docker volumes.
    reset: bool,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            platform_url: std::env::var("PLATFORM_DATABASE_URL")
                .context("PLATFORM_DATABASE_URL must be set")?,
            agency_url: std::env::var("AGENCY_DATABASE_URL")
                .context("AGENCY_DATABASE_URL must be set")?,
            reset: std::env::var("MIGRATE_RESET").unwrap_or_default() == "true",
        })
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::from_env()?;

    if cfg.reset {
        println!("⚠  MIGRATE_RESET=true — all schemas will be dropped and rebuilt");
    }

    // ── Step 1: Platform DB ───────────────────────────────────────────────────
    run_platform_migrations(&cfg).await?;

    // ── Step 2: Agency DB (per-schema) ────────────────────────────────────────
    run_agency_migrations(&cfg).await?;

    println!();
    println!("✓ Migration complete");
    Ok(())
}

// ── Platform ──────────────────────────────────────────────────────────────────

async fn run_platform_migrations(cfg: &Config) -> Result<()> {
    println!();
    println!("── Platform DB ──────────────────────────────────────────");

    let pool = make_pool(&cfg.platform_url, "public")
        .await
        .context("Failed to connect to platform database")?;
    println!("✓ Connected to platform database");

    if cfg.reset {
        println!("  → Dropping public schema…");
        pool.execute("DROP SCHEMA IF EXISTS public CASCADE").await?;
        pool.execute("CREATE SCHEMA public").await?;
        println!("  ✓ Platform schema cleared");
    }

    println!("  → Running platform migrations…");
    PLATFORM_MIGRATOR
        .run(&pool)
        .await
        .context("Platform migrations failed")?;
    println!("  ✓ Platform migrations complete");

    println!("  → Seeding platform reference data (plans, features, limits)…");
    seed::run_platform_seeds(&pool).await?;
    println!("  ✓ Platform seeds complete");

    pool.close().await;
    Ok(())
}

// ── Agency ────────────────────────────────────────────────────────────────────

async fn run_agency_migrations(cfg: &Config) -> Result<()> {
    println!();
    println!("── Agency DB ────────────────────────────────────────────");

    // Re-open platform pool to read the agencies table
    let platform_pool = make_pool(&cfg.platform_url, "public")
        .await
        .context("Failed to re-connect to platform database")?;

    let agency_base_pool = make_pool(&cfg.agency_url, "public")
        .await
        .context("Failed to connect to agency database")?;
    println!("✓ Connected to agency database");

    let schemas: Vec<(String,)> =
        sqlx::query_as("SELECT schema_name FROM agencies ORDER BY created_at")
            .fetch_all(&platform_pool)
            .await
            .context("Failed to fetch agencies from platform DB")?;

    platform_pool.close().await;

    if schemas.is_empty() {
        println!("  → No agencies yet — create one via POST /api/v1/admin/agencies");
    } else {
        for (schema,) in &schemas {
            if cfg.reset {
                drop_schema(&agency_base_pool, schema).await?;
            }
            migrate_agency_schema(&cfg.agency_url, schema).await?;
        }
        println!("  ✓ All {} agency schema(s) migrated", schemas.len());
    }

    agency_base_pool.close().await;
    Ok(())
}

// ── Core helpers ──────────────────────────────────────────────────────────────

/// Opens a pool whose every connection starts with `SET search_path TO <schema>, public`.
async fn make_pool(url: &str, schema: &str) -> Result<PgPool> {
    let schema = schema.to_owned();
    PgPoolOptions::new()
        .max_connections(2)
        .after_connect(move |conn, _| {
            let s = schema.clone();
            Box::pin(async move {
                conn.execute(format!("SET search_path TO {s}, public").as_str())
                    .await?;
                Ok(())
            })
        })
        .connect(url)
        .await
        .with_context(|| format!("Failed to open pool for {url}"))
}

/// Creates the schema (if missing) then runs AGENCY_MIGRATOR against it.
async fn migrate_agency_schema(db_url: &str, schema_name: &str) -> Result<()> {
    println!("  → Migrating agency schema '{schema_name}'…");

    let pool = make_pool(db_url, schema_name)
        .await
        .with_context(|| format!("Failed to connect for schema '{schema_name}'"))?;

    pool.execute(format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\"").as_str())
        .await
        .with_context(|| format!("Failed to create schema '{schema_name}'"))?;

    AGENCY_MIGRATOR
        .run(&pool)
        .await
        .with_context(|| format!("Agency migrations failed for schema '{schema_name}'"))?;

    pool.close().await;
    println!("  ✓ Schema '{schema_name}' done");
    Ok(())
}

async fn drop_schema(pool: &PgPool, schema_name: &str) -> Result<()> {
    println!("  → Dropping schema '{schema_name}'…");
    pool.execute(format!("DROP SCHEMA IF EXISTS \"{schema_name}\" CASCADE").as_str())
        .await
        .with_context(|| format!("Failed to drop schema '{schema_name}'"))?;
    Ok(())
}
