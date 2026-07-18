//
// Platform seed data.  Lives here so the `migrate` binary is fully
// self-contained: no dependency on the main `emakao` crate, no compile-time
// SQL macros.  Every query uses plain `sqlx::query(...)` + `.bind()`.
//
// All functions are idempotent — safe to call on every `make migrate` run.
// Plans use the table's DEFAULT `uuid_generate_v4()` for their primary key so
// the data is portable across fresh databases without hard-coded UUIDs.

use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Seed everything the platform DB needs before the first agency is created.
/// Must be called after platform migrations have run successfully.
pub async fn run_platform_seeds(pool: &PgPool) -> Result<()> {
    seed_subscription_plans(pool).await?;
    seed_plan_features(pool).await?;
    seed_plan_limits(pool).await?;
    Ok(())
}

pub async fn run_agency_seeds(_pool: &PgPool) -> Result<()> {
    // Add any agency-specific seed data here in the future
    Ok(())
}

struct PlanSeed {
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    price_kes: i32,
    yearly_price_kes: i32,
    trial_days: i32,
    sort_order: i32,
}

async fn seed_subscription_plans(pool: &PgPool) -> Result<()> {
    const PLANS: &[PlanSeed] = &[
        PlanSeed {
            slug: "professional",
            name: "Professional Tier",
            description: "Essential workflow organization and manual financial tracking for growing portfolios.",
            price_kes: 5_000,
            yearly_price_kes: 50_000,
            trial_days: 14,
            sort_order: 1,
        },
        PlanSeed {
            slug: "enterprise",
            name: "Enterprise Compliance Engine",
            description: "Automated eTIMS generation, eRITS exports, WHT compliance, and direct M-Pesa automated reconciliation.",
            price_kes: 19_500,
            yearly_price_kes: 195_000,
            trial_days: 14,
            sort_order: 2,
        },
    ];

    for p in PLANS {
        sqlx::query(
            r#"
            INSERT INTO subscription_plans 
                (slug, name, description, price_kes, yearly_price_kes, "interval", trial_days, is_active, is_public, sort_order)
            VALUES ($1, $2, $3, $4, $5, 'monthly', $6, true, true, $7)
            ON CONFLICT (slug) DO UPDATE SET
                name = EXCLUDED.name, description = EXCLUDED.description,
                price_kes = EXCLUDED.price_kes, yearly_price_kes = EXCLUDED.yearly_price_kes,
                sort_order = EXCLUDED.sort_order, updated_at = now()
            "#
        )
        .bind(p.slug)
        .bind(p.name)
        .bind(p.description)
        .bind(p.price_kes)
        .bind(p.yearly_price_kes)
        .bind(p.trial_days)
        .bind(p.sort_order)
        .execute(pool)
        .await
        .context("Failed to seed subscription plans")?;
    }
    Ok(())
}

struct FeatureSeed {
    plan: &'static str,
    key: &'static str,
    enabled: bool,
}

async fn seed_plan_features(pool: &PgPool) -> Result<()> {
    const FEATURES: &[FeatureSeed] = &[
        // Professional Tier Core Features
        FeatureSeed {
            plan: "professional",
            key: "portal_resident",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "portal_resident",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "portal_caretaker",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "portal_caretaker",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "portal_owner",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "portal_owner",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "maint_work_orders",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "maint_work_orders",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "utility_billing",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "utility_billing",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "utility_meter_reading",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "utility_meter_reading",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "payment_mpesa_recon",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "payment_mpesa_recon",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "acct_standard_ledger",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "acct_standard_ledger",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "comm_sms_automation",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "comm_sms_automation",
            enabled: true,
        },
        // Enterprise Compliance & Automation Suite
        FeatureSeed {
            plan: "professional",
            key: "payment_bulk_disbursements",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "payment_bulk_disbursements",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "acct_double_entry",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "acct_double_entry",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "acct_kra_compliance",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "acct_kra_compliance",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "comm_whatsapp_automation",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "comm_whatsapp_automation",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "portal_vendor",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "portal_vendor",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "core_multi_branch",
            enabled: true,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "core_multi_branch",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "core_custom_roles",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "core_custom_roles",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "core_audit_log",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "core_audit_log",
            enabled: true,
        },
        FeatureSeed {
            plan: "professional",
            key: "brand_white_label",
            enabled: false,
        },
        FeatureSeed {
            plan: "enterprise",
            key: "brand_white_label",
            enabled: true,
        },
    ];

    for f in FEATURES {
        sqlx::query(
            r#"
            INSERT INTO plan_features (plan_id, feature_key, value, value_type, enabled)
            SELECT p.id, $2, $3, 'boolean', $4 FROM subscription_plans p WHERE p.slug = $1
            ON CONFLICT (plan_id, feature_key) DO UPDATE SET value = EXCLUDED.value, enabled = EXCLUDED.enabled
            "#
        )
        .bind(f.plan)
        .bind(f.key)
        .bind(if f.enabled { "true" } else { "false" })
        .bind(f.enabled)
        .execute(pool)
        .await?;
    }
    Ok(())
}

struct LimitSeed {
    plan: &'static str,
    key: &'static str,
    max_value: i32,
    soft_limit: Option<i32>,
}

async fn seed_plan_limits(pool: &PgPool) -> Result<()> {
    const LIMITS: &[LimitSeed] = &[
        // Professional (Units dropped to 150 based on KES 5,000 price point)
        LimitSeed {
            plan: "professional",
            key: "max_units",
            max_value: 150,
            soft_limit: Some(135),
        },
        LimitSeed {
            plan: "professional",
            key: "max_branches",
            max_value: 10,
            soft_limit: Some(8),
        },
        LimitSeed {
            plan: "professional",
            key: "max_storage_mb",
            max_value: 2048,
            soft_limit: Some(1800),
        },
        LimitSeed {
            plan: "professional",
            key: "max_sms_per_month",
            max_value: 1000,
            soft_limit: Some(900),
        },
        LimitSeed {
            plan: "professional",
            key: "max_whatsapp_per_month",
            max_value: 0,
            soft_limit: Some(0),
        },
        // Enterprise (-1 = unlimited)
        LimitSeed {
            plan: "enterprise",
            key: "max_units",
            max_value: -1,
            soft_limit: None,
        },
        LimitSeed {
            plan: "enterprise",
            key: "max_branches",
            max_value: -1,
            soft_limit: None,
        },
        LimitSeed {
            plan: "enterprise",
            key: "max_storage_mb",
            max_value: 51200,
            soft_limit: Some(46080),
        },
        LimitSeed {
            plan: "enterprise",
            key: "max_sms_per_month",
            max_value: 10000,
            soft_limit: Some(9000),
        },
        LimitSeed {
            plan: "enterprise",
            key: "max_whatsapp_per_month",
            max_value: 5000,
            soft_limit: Some(4500),
        },
    ];

    for l in LIMITS {
        sqlx::query(
            r#"
            INSERT INTO plan_limits (plan_id, limit_key, max_value, soft_limit)
            SELECT p.id, $2, $3, $4 FROM subscription_plans p WHERE p.slug = $1
            ON CONFLICT (plan_id, limit_key) DO UPDATE SET max_value = EXCLUDED.max_value, soft_limit = EXCLUDED.soft_limit
            "#
        )
        .bind(l.plan)
        .bind(l.key)
        .bind(l.max_value)
        .bind(l.soft_limit)
        .execute(pool)
        .await?;
    }
    Ok(())
}

struct ChecklistItemSeed {
    name: &'static str,
    description: Option<&'static str>,
    checklist_type: &'static str, // "move-in", "move-out", "both"
    sort_order: i32,
}

async fn seed_default_checklist(pool: &PgPool) -> Result<()> {
    // First, create the checklist
    let checklist_id_opt: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO checklists (id, agency_id, name, description, is_default, created_at, updated_at)
        VALUES (uuid_generate_v7(), '00000000-0000-0000-0000-000000000000', $1, $2, true, now(), now())
        ON CONFLICT DO NOTHING
        RETURNING id
        "#
    )
    .bind("Kenyan Market Move-In/Move-Out Checklist")
    .bind(Some("Standard checklist tailored for Kenyan property management"))
    .fetch_optional(pool)
    .await?;

    let checklist_id = match checklist_id_opt {
        Some(id) => id,
        None => {
            // Get existing checklist ID if already seeded
            sqlx::query_scalar(
                r#"
                SELECT id FROM checklists WHERE is_default = true
                "#,
            )
            .fetch_one(pool)
            .await?
        }
    };

    // Section 1: Entry, Hallways & Common Areas
    let section1_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
        VALUES (uuid_generate_v7(), $1, $2, $3, $4, now(), now())
        ON CONFLICT (checklist_id, name) DO NOTHING
        RETURNING id
        "#
    )
    .bind(checklist_id)
    .bind("Entry, Hallways & Common Areas")
    .bind(Some("The first zone any tenant or inspector encounters sets the documentation baseline. Entry damage — scratched frames, scuffed walls, broken hardware — is frequently overlooked at move-in but heavily contested at move-out."))
    .bind(1)
    .fetch_optional(pool)
    .await?;

    if let Some(section_id) = section1_id {
        for item in &[
            ChecklistItemSeed { name: "Inspect Front Door Condition and Lock Functionality", description: Some("Test the deadbolt, knob lock, and any chain or slide mechanisms for smooth operation. Document scratches, dents, or paint chipping on the door face and frame. Confirm all provided keys operate the lock without sticking — a lock that binds at move-in will be reported as damaged at move-out."), checklist_type: "both", sort_order: 1 },
            ChecklistItemSeed { name: "Check Walls and Baseboards for Pre-Existing Damage", description: Some("Walk every wall surface in hallways and entryways, noting scuffs, nail holes, crayon marks, water stains, or peeling paint. Photograph any damage from multiple angles with a timestamp. Baseboards are frequently scuffed from furniture moves — document condition before the tenant takes possession."), checklist_type: "move-in", sort_order: 2 },
            ChecklistItemSeed { name: "Verify Flooring Condition and Note Existing Wear", description: Some("Inspect hardwood, tile, or carpet in entryways for scratches, staining, lifted edges, or missing grout. Mark specific areas on a unit diagram to clearly distinguish pre-existing wear from tenant-caused damage. Carpet with existing staining must be photographed before move-in to avoid deposit deduction disputes later."), checklist_type: "both", sort_order: 3 },
            ChecklistItemSeed { name: "Test Light Fixtures and Switches in Common Spaces", description: Some("Operate all switches and confirm lights illuminate without flickering. Note any missing bulbs, cracked cover plates, or fixtures with loose housing. Faulty switches at move-in should be repaired before occupancy — not left as open items that create ambiguity at move-out."), checklist_type: "both", sort_order: 4 },
        ] {
            sqlx::query(
                r#"
                INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
                VALUES (uuid_generate_v7(), $1, $2, $3, $4::checklist_type, $5, now(), now())
                ON CONFLICT DO NOTHING
                "#
            )
            .bind(section_id)
            .bind(item.name)
            .bind(item.description)
            .bind(item.checklist_type)
            .bind(item.sort_order)
            .execute(pool)
            .await?;
        }
    }

    // Section 2: Living Room & Dining Area
    let section2_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
        VALUES (uuid_generate_v7(), $1, $2, $3, $4, now(), now())
        ON CONFLICT (checklist_id, name) DO NOTHING
        RETURNING id
        "#
    )
    .bind(checklist_id)
    .bind("Living Room & Dining Area")
    .bind(Some("High-traffic living spaces accumulate the most tenant-generated wear. A thorough baseline inspection here reduces the most common security deposit disputes — wall damage, flooring wear, and window condition — to documented fact rather than estimation."))
    .bind(2)
    .fetch_optional(pool)
    .await?;

    if let Some(section_id) = section2_id {
        for item in &[
            ChecklistItemSeed { name: "Document All Wall Surfaces Including Paint Condition", description: Some("Photograph every wall surface from corner to corner, noting existing holes, chips, crayon or marker damage, moisture bubbling, or improperly patched areas. Record the paint color and sheen on the inspection report — if the landlord must repaint specific areas at move-out, move-in documentation is essential proof."), checklist_type: "move-in", sort_order: 1 },
            ChecklistItemSeed { name: "Inspect Windows, Sills, and Blinds for Existing Damage", description: Some("Open and close every window to verify operation and check locks. Note any cracked panes, broken tilt mechanisms on blinds, torn screens, or damaged sill paint. Blinds and screens are among the most frequently disputed items at move-out — their documented move-in condition determines what is tenant responsibility."), checklist_type: "both", sort_order: 2 },
            ChecklistItemSeed { name: "Test Electrical Outlets and GFCI Devices", description: Some("Use an outlet tester or phone charger to confirm every outlet in the living and dining space is live and grounded. Test GFCI outlets with the test/reset buttons. Dead outlets at move-in that go undocumented may be attributed to tenant damage — a quick circuit test prevents that assumption."), checklist_type: "move-in", sort_order: 3 },
            ChecklistItemSeed { name: "Check Ceiling for Water Stains, Cracks, or Texture Damage", description: Some("Inspect the ceiling surface under lighting for water rings, hairline cracks, peeling texture, or staining from prior leaks. Any ceiling damage must be photographed and noted — ceiling repairs are expensive, and their cause is difficult to determine after the fact without a documented baseline."), checklist_type: "both", sort_order: 4 },
        ] {
            sqlx::query(
                r#"
                INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
                VALUES (uuid_generate_v7(), $1, $2, $3, $4::checklist_type, $5, now(), now())
                ON CONFLICT DO NOTHING
                "#
            )
            .bind(section_id)
            .bind(item.name)
            .bind(item.description)
            .bind(item.checklist_type)
            .bind(item.sort_order)
            .execute(pool)
            .await?;
        }
    }

    // Section 3: Kitchen Inspection
    let section3_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
        VALUES (uuid_generate_v7(), $1, $2, $3, $4, now(), now())
        ON CONFLICT (checklist_id, name) DO NOTHING
        RETURNING id
        "#
    )
    .bind(checklist_id)
    .bind("Kitchen Inspection")
    .bind(Some("Kitchens generate the highest maintenance costs at tenant turnover. Appliance condition, cabinetry, countertops, and plumbing must all be documented at move-in to accurately attribute damage charges and avoid deductions for pre-existing wear."))
    .bind(3)
    .fetch_optional(pool)
    .await?;

    if let Some(section_id) = section3_id {
        for item in &[
            ChecklistItemSeed { name: "Test All Kitchen Appliances and Document Operation", description: Some("Operate the stove burners, oven, dishwasher, refrigerator, and microwave to confirm each functions as intended. Note any error codes, broken knobs, missing racks, or cracked door seals. Photograph the interior of the refrigerator and oven — grease buildup or ice accumulation at move-in must be on record."), checklist_type: "both", sort_order: 1 },
            ChecklistItemSeed { name: "Inspect Countertops and Cabinets for Chips, Burns, or Staining", description: Some("Photograph countertop surfaces for any existing burns, chips, discoloration, or delamination. Open every cabinet and drawer to note broken hinges, missing hardware, and drawer slide operation. These items are commonly charged at move-out without move-in documentation to dispute the claim."), checklist_type: "move-in", sort_order: 2 },
            ChecklistItemSeed { name: "Check Sink, Faucet, and Under-Sink Plumbing for Leaks", description: Some("Run the kitchen faucet at full pressure and inspect the P-trap and supply lines under the sink for moisture, corrosion, or drips. A slow leak under the kitchen sink at move-in creates mold and cabinet floor damage over time — document and repair before occupancy begins."), checklist_type: "move-in", sort_order: 3 },
            ChecklistItemSeed { name: "Verify Appliance Cleaning Standard Before Key Handover", description: Some("Inspect oven interior, refrigerator shelves, dishwasher filter, and microwave cavity against the cleaning standard specified in the lease. Note any areas requiring professional cleaning on the move-out report with photographs. Cleaning charges must be clearly distinguished from damage charges in the deposit accounting statement."), checklist_type: "move-out", sort_order: 4 },
        ] {
            sqlx::query(
                r#"
                INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
                VALUES (uuid_generate_v7(), $1, $2, $3, $4::checklist_type, $5, now(), now())
                ON CONFLICT DO NOTHING
                "#
            )
            .bind(section_id)
            .bind(item.name)
            .bind(item.description)
            .bind(item.checklist_type)
            .bind(item.sort_order)
            .execute(pool)
            .await?;
        }
    }

    // Section 4: Bathrooms Inspection
    let section4_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
        VALUES (uuid_generate_v7(), $1, $2, $3, $4, now(), now())
        ON CONFLICT (checklist_id, name) DO NOTHING
        RETURNING id
        "#
    )
    .bind(checklist_id)
    .bind("Bathrooms Inspection")
    .bind(Some("Bathrooms are the highest-risk area for pre-existing water damage claims. Grout, caulk, fixtures, and exhaust ventilation must all be documented at move-in because moisture damage accumulates silently and disputes about its origin arise almost universally at move-out."))
    .bind(4)
    .fetch_optional(pool)
    .await?;

    if let Some(section_id) = section4_id {
        for item in &[
            ChecklistItemSeed { name: "Document Grout and Caulk Condition in Shower and Tub Areas", description: Some("Photograph grout lines in tiled shower and tub surrounds to capture any existing cracking, discoloration, or missing sections. Check caulk beads at the tub-wall transitions for gaps, mold staining, or separation. Failed caulk at move-in that is not repaired becomes a water damage liability during the tenancy."), checklist_type: "move-in", sort_order: 1 },
            ChecklistItemSeed { name: "Test Toilet for Flushing, Rocking, and Tank Leaks", description: Some("Flush the toilet and observe the fill cycle for proper shut-off. Check for rocking at the base indicating a compromised wax ring. Add food dye to the tank and check the bowl after five minutes for a silent flapper leak. Document any existing cracks in the porcelain or loose toilet seat hardware."), checklist_type: "both", sort_order: 2 },
            ChecklistItemSeed { name: "Inspect Vanity, Mirror, and Medicine Cabinet Condition", description: Some("Open the medicine cabinet and vanity drawers to check hinge alignment. Note any chips in the sink basin, cracks in the vanity top, or fogging on the mirror backing. Photograph the underside of the vanity for water damage from prior leaks before the tenant takes possession."), checklist_type: "move-in", sort_order: 3 },
            ChecklistItemSeed { name: "Test Bathroom Exhaust Fan and Confirm Exterior Venting", description: Some("Switch on the bathroom exhaust fan and hold a tissue near the grille to confirm airflow. Confirm the fan exhausts to the exterior of the building. A non-functional bathroom fan leads to moisture accumulation and mold — document its operational status at move-in and repair it before the tenancy begins."), checklist_type: "move-in", sort_order: 4 },
        ] {
            sqlx::query(
                r#"
                INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
                VALUES (uuid_generate_v7(), $1, $2, $3, $4::checklist_type, $5, now(), now())
                ON CONFLICT DO NOTHING
                "#
            )
            .bind(section_id)
            .bind(item.name)
            .bind(item.description)
            .bind(item.checklist_type)
            .bind(item.sort_order)
            .execute(pool)
            .await?;
        }
    }

    // Section 5: Bedrooms Inspection
    let section5_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
        VALUES (uuid_generate_v7(), $1, $2, $3, $4, now(), now())
        ON CONFLICT (checklist_id, name) DO NOTHING
        RETURNING id
        "#
    )
    .bind(checklist_id)
    .bind("Bedrooms Inspection")
    .bind(Some("Bedroom documentation is frequently skipped in informal inspections, yet closet damage, wall scuffing, and window operation issues are among the most disputed items at move-out. Every bedroom requires its own documented walkthrough."))
    .bind(5)
    .fetch_optional(pool)
    .await?;

    if let Some(section_id) = section5_id {
        for item in &[
            ChecklistItemSeed { name: "Document Closet Interior Condition Including Rods and Shelving", description: Some("Photograph each closet interior with the door open, noting any bent rods, damaged shelving brackets, broken bifold door hardware, or mold near exterior walls. Closet wall surfaces should be inspected for moisture staining, particularly in corner rooms where condensation risk is higher."), checklist_type: "both", sort_order: 1 },
            ChecklistItemSeed { name: "Inspect Bedroom Doors, Hinges, and Door Hardware", description: Some("Open and close each bedroom door to test for proper latch operation and swing clearance. Check hinge tightness and note any existing door frame gouges. Doors that don't latch properly should be adjusted before move-in — an improperly operating door will be reported as damaged by any careful tenant."), checklist_type: "both", sort_order: 2 },
            ChecklistItemSeed { name: "Check Carpet or Flooring for Stains, Tears, or Worn Areas", description: Some("Inspect bedroom flooring in natural light for existing stains, bleach spots, worn traffic paths, or lifted carpet edges at transition strips. Carpet near windows and closet entries is typically the most worn — document it in detail to support fair deposit accounting at move-out."), checklist_type: "both", sort_order: 3 },
            ChecklistItemSeed { name: "Verify Window Operation, Screen Integrity, and Lock Function", description: Some("Open and close every bedroom window to confirm the sash slides freely and the latch locks securely. Inspect screens for tears or bent frames. Note any fogged double-pane glass or cracked glazing — window seal failures are a building maintenance responsibility that should never be charged to a tenant at move-out."), checklist_type: "both", sort_order: 4 },
        ] {
            sqlx::query(
                r#"
                INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
                VALUES (uuid_generate_v7(), $1, $2, $3, $4::checklist_type, $5, now(), now())
                ON CONFLICT DO NOTHING
                "#
            )
            .bind(section_id)
            .bind(item.name)
            .bind(item.description)
            .bind(item.checklist_type)
            .bind(item.sort_order)
            .execute(pool)
            .await?;
        }
    }

    // Section 6: Utilities, HVAC & Safety Systems
    let section6_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
        VALUES (uuid_generate_v7(), $1, $2, $3, $4, now(), now())
        ON CONFLICT (checklist_id, name) DO NOTHING
        RETURNING id
        "#
    )
    .bind(checklist_id)
    .bind("Utilities, HVAC & Safety Systems")
    .bind(Some("HVAC systems, smoke detectors, carbon monoxide alarms, and utility shut-offs must be documented and verified operational at move-in — not just for deposit protection, but as a legal obligation in most jurisdictions."))
    .bind(6)
    .fetch_optional(pool)
    .await?;

    if let Some(section_id) = section6_id {
        for item in &[
            ChecklistItemSeed { name: "Test Smoke Detectors and Carbon Monoxide Alarms", description: Some("Press the test button on every smoke detector and CO alarm and confirm the audible alarm activates. Note the manufacture date on each device — detectors older than ten years must be replaced before occupancy in most jurisdictions. Document serial numbers and test dates on the inspection report."), checklist_type: "move-in", sort_order: 1 },
            ChecklistItemSeed { name: "Inspect HVAC Filter, Thermostat, and Supply Vents", description: Some("Remove and photograph the HVAC air filter to document its condition at move-in — a heavily loaded filter on day one is a maintenance responsibility, not a tenant obligation. Test the thermostat in both heat and cool modes and confirm supply vents in each room are delivering airflow."), checklist_type: "both", sort_order: 2 },
            ChecklistItemSeed { name: "Record Utility Meter Readings", description: Some("Photograph electric, gas, and water meter readings with a timestamp on the day of key handover for both move-in and move-out. Meter readings prevent billing disputes for utility charges that span the turnover period and provide an objective reference for any utility-based damage claims."), checklist_type: "both", sort_order: 3 },
            ChecklistItemSeed { name: "Locate and Document Utility Shut-Off Valve Locations", description: Some("Confirm the tenant is aware of the main water shut-off, electrical panel location and breaker labeling, and gas shut-off valve location. Note the condition of the breaker panel cover and accuracy of circuit labels. Proper tenant orientation at move-in reduces emergency response time for utility incidents."), checklist_type: "move-in", sort_order: 4 },
        ] {
            sqlx::query(
                r#"
                INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
                VALUES (uuid_generate_v7(), $1, $2, $3, $4::checklist_type, $5, now(), now())
                ON CONFLICT DO NOTHING
                "#
            )
            .bind(section_id)
            .bind(item.name)
            .bind(item.description)
            .bind(item.checklist_type)
            .bind(item.sort_order)
            .execute(pool)
            .await?;
        }
    }

    // Section 7: Move-Out: Final Condition Verification
    let section7_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        INSERT INTO checklist_sections (id, checklist_id, name, description, sort_order, created_at, updated_at)
        VALUES (uuid_generate_v7(), $1, $2, $3, $4, now(), now())
        ON CONFLICT (checklist_id, name) DO NOTHING
        RETURNING id
        "#
    )
    .bind(checklist_id)
    .bind("Move-Out: Final Condition Verification")
    .bind(Some("The move-out inspection should mirror the move-in walkthrough room by room, using the original report as the comparison baseline. A structured move-out process ensures deposit deductions are defensible, documented, and legally sound."))
    .bind(7)
    .fetch_optional(pool)
    .await?;

    if let Some(section_id) = section7_id {
        for item in &[
            ChecklistItemSeed { name: "Compare Move-Out Condition with Move-In Report Side-by-Side", description: Some("Bring the completed move-in inspection report to the move-out walkthrough and photograph each area from the same angle used at move-in. Side-by-side comparison photos provide unambiguous documentation of new damage versus pre-existing condition — the strongest evidence in any deposit dispute."), checklist_type: "move-out", sort_order: 1 },
            ChecklistItemSeed { name: "Verify Cleaning Obligations Are Met Before Key Return", description: Some("Inspect appliance interiors, bathroom surfaces, cabinet interiors, and flooring against the cleaning standard specified in the lease. Note any areas requiring professional cleaning on the move-out report with photographs. Clearly distinguish cleaning charges from damage charges in the deposit accounting statement."), checklist_type: "move-out", sort_order: 2 },
            ChecklistItemSeed { name: "Confirm All Keys, Remotes, and Access Devices Are Returned", description: Some("Collect every key, fob, parking remote, garage opener, and mailbox key provided at move-in. Cross-reference the move-in key receipt log. Unreturned keys require lock rekeying — the cost must be supported by documentation showing keys were provided at move-in and not returned at move-out."), checklist_type: "move-out", sort_order: 3 },
            ChecklistItemSeed { name: "Issue Itemized Deposit Accounting Within Statutory Deadline", description: Some("Prepare an itemized written statement of any security deposit deductions with supporting repair estimates or receipts, and deliver it within the timeline required by local landlord-tenant law — typically 14 to 30 days from move-out. Late or undocumented deposit accounting forfeits the right to deductions in most states."), checklist_type: "move-out", sort_order: 4 },
        ] {
            sqlx::query(
                r#"
                INSERT INTO checklist_items (id, section_id, name, description, checklist_type, sort_order, created_at, updated_at)
                VALUES (uuid_generate_v7(), $1, $2, $3, $4::checklist_type, $5, now(), now())
                ON CONFLICT DO NOTHING
                "#
            )
            .bind(section_id)
            .bind(item.name)
            .bind(item.description)
            .bind(item.checklist_type)
            .bind(item.sort_order)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}
