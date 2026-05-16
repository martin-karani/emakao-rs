// src/presentation/http/dto/dashboard.rs
//
// Dashboard query params are defined inside the handler (DashboardParams)
// because they are simple enough to live there. This module exists so the
// dto::mod.rs re-export is consistent and future per-property filter params
// can be added here without touching the handler.
