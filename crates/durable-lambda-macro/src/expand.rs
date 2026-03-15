//! Macro expansion logic — code generation for #[durable_execution].
//!
//! Generates lambda_runtime registration + DurableContext setup,
//! mirroring the #[tokio::main] pattern.
