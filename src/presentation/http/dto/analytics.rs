//
// Query parameters for analytics endpoints are defined directly
// inside the handler file (analytics.rs) as `AnalyticsParams`,
// because they carry `time::Date` fields that need the utoipa
// IntoParams derive close to the handler.
//
// This module is a placeholder for any future request-body DTOs,
// such as a scheduled-export configuration payload.
