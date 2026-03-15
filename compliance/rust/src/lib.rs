//! Python/Rust behavioral compliance verification.
//!
//! This crate contains Rust implementations of workflows that mirror
//! Python reference implementations. The compliance test harness verifies
//! that both produce identical operation sequences.

pub mod callback_approval;
pub mod order_processing;
pub mod parallel_fanout;
