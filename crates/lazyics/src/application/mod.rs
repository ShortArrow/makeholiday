//! Application layer. Re-exports `icscli`'s use cases so screens drive the
//! same `add` / `edit` / `remove` logic as the CLI (ADR-025).

pub use icscli::application::use_cases;
