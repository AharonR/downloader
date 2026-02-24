//! Critical test matrix (Phases 1–5): data integrity, network resilience, auth security,
//! resource management, system recovery.
//!
//! Run with: `cargo test --test critical`
//! Run Phase 1–3 (P0) for pre-merge; Phase 4–5 (P1) can be run with `--ignored` for nightly.

mod support;

#[path = "critical/data_corruption.rs"]
mod data_corruption;
#[path = "critical/persistence_recovery.rs"]
mod persistence_recovery;
#[path = "critical/race_conditions.rs"]
mod race_conditions;
#[path = "critical/transaction_failures.rs"]
mod transaction_failures;

#[path = "critical/intermittent_connectivity.rs"]
mod intermittent_connectivity;
#[path = "critical/network_failures.rs"]
mod network_failures;
#[path = "critical/rate_limit_handling.rs"]
mod rate_limit_handling;
#[path = "critical/timeout_edge_cases.rs"]
mod timeout_edge_cases;

#[path = "critical/auth_bypass.rs"]
mod auth_bypass;
#[path = "critical/cookie_poisoning.rs"]
mod cookie_poisoning;
#[path = "critical/credential_leakage.rs"]
mod credential_leakage;
#[path = "critical/encryption_failures.rs"]
mod encryption_failures;

#[path = "critical/concurrent_load.rs"]
mod concurrent_load;
#[path = "critical/disk_space_failures.rs"]
mod disk_space_failures;
#[path = "critical/file_descriptor_exhaustion.rs"]
mod file_descriptor_exhaustion;
#[path = "critical/memory_leaks.rs"]
mod memory_leaks;

#[path = "critical/corrupted_state.rs"]
mod corrupted_state;
#[path = "critical/crash_recovery.rs"]
mod crash_recovery;
#[path = "critical/interrupted_operations.rs"]
mod interrupted_operations;
#[path = "critical/power_failure_simulation.rs"]
mod power_failure_simulation;
