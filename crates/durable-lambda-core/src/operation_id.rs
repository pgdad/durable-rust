//! Deterministic operation ID generation for replay correctness.
//!
//! Operation IDs must be identical across replays for the same code path.
//! This module implements the Python SDK's ID generation strategy:
//! `blake2b("{counter}")` for root operations, `blake2b("{parent_id}-{counter}")`
//! for child operations, truncated to 64 hex characters.

use blake2::{Blake2b512, Digest};

/// Generate deterministic operation IDs for durable operations.
///
/// Each `OperationIdGenerator` maintains a monotonically increasing counter.
/// IDs are computed as `blake2b(input)` truncated to 64 hex characters, where
/// `input` is `"{counter}"` for root operations or `"{parent_id}-{counter}"`
/// for child operations.
///
/// # Determinism Invariant
///
/// The same code path executing the same sequence of durable operations
/// **must** produce the same operation IDs across replays. This is the
/// fundamental correctness requirement for the replay engine.
///
/// # Examples
///
/// ```
/// use durable_lambda_core::operation_id::OperationIdGenerator;
///
/// let mut gen = OperationIdGenerator::new(None);
/// let id1 = gen.next_id();
/// let id2 = gen.next_id();
///
/// // IDs are deterministic — same counter produces same ID.
/// let mut gen2 = OperationIdGenerator::new(None);
/// assert_eq!(id1, gen2.next_id());
/// assert_eq!(id2, gen2.next_id());
///
/// // Different IDs for different counters.
/// assert_ne!(id1, id2);
/// ```
#[derive(Debug, Clone)]
pub struct OperationIdGenerator {
    counter: u64,
    parent_id: Option<String>,
}

impl OperationIdGenerator {
    /// Create a new generator, optionally scoped under a parent operation.
    ///
    /// - `parent_id: None` — root-level generator, hashes `"{counter}"`
    /// - `parent_id: Some(id)` — child generator, hashes `"{parent_id}-{counter}"`
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::operation_id::OperationIdGenerator;
    ///
    /// // Root-level generator.
    /// let root = OperationIdGenerator::new(None);
    ///
    /// // Child generator scoped to a parent operation.
    /// let child = OperationIdGenerator::new(Some("abc123".to_string()));
    /// ```
    pub fn new(parent_id: Option<String>) -> Self {
        Self {
            counter: 0,
            parent_id,
        }
    }

    /// Generate the next deterministic operation ID.
    ///
    /// Increments the internal counter and returns a 64-character hex string
    /// derived from `blake2b` hashing.
    ///
    /// # Examples
    ///
    /// ```
    /// use durable_lambda_core::operation_id::OperationIdGenerator;
    ///
    /// let mut gen = OperationIdGenerator::new(None);
    /// let id = gen.next_id();
    /// assert_eq!(id.len(), 64);
    /// ```
    pub fn next_id(&mut self) -> String {
        self.counter += 1;
        let input = match &self.parent_id {
            Some(parent) => format!("{}-{}", parent, self.counter),
            None => self.counter.to_string(),
        };
        blake2b_hash_64(&input)
    }
}

/// Compute blake2b hash of input, returning first 64 hex characters.
fn blake2b_hash_64(input: &str) -> String {
    let mut hasher = Blake2b512::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let full_hex = hex::encode(result);
    full_hex[..64].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_id_is_64_hex_chars() {
        let mut gen = OperationIdGenerator::new(None);
        let id = gen.next_id();
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn root_ids_are_deterministic() {
        let mut gen1 = OperationIdGenerator::new(None);
        let mut gen2 = OperationIdGenerator::new(None);

        for _ in 0..5 {
            assert_eq!(gen1.next_id(), gen2.next_id());
        }
    }

    #[test]
    fn root_ids_are_unique() {
        let mut gen = OperationIdGenerator::new(None);
        let ids: Vec<String> = (0..10).map(|_| gen.next_id()).collect();

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "IDs at {} and {} should differ", i, j);
            }
        }
    }

    #[test]
    fn child_ids_differ_from_root() {
        let mut root = OperationIdGenerator::new(None);
        let mut child = OperationIdGenerator::new(Some("parent123".to_string()));

        // Same counter value but different parent → different IDs.
        assert_ne!(root.next_id(), child.next_id());
    }

    #[test]
    fn child_ids_are_deterministic() {
        let parent = "my-parent-id".to_string();
        let mut gen1 = OperationIdGenerator::new(Some(parent.clone()));
        let mut gen2 = OperationIdGenerator::new(Some(parent));

        for _ in 0..5 {
            assert_eq!(gen1.next_id(), gen2.next_id());
        }
    }

    #[test]
    fn different_parents_produce_different_ids() {
        let mut gen_a = OperationIdGenerator::new(Some("parent-a".to_string()));
        let mut gen_b = OperationIdGenerator::new(Some("parent-b".to_string()));

        assert_ne!(gen_a.next_id(), gen_b.next_id());
    }

    #[test]
    fn counter_increments_correctly() {
        // Verify the hash input is "{counter}" for root:
        // counter=1 → hash("1"), counter=2 → hash("2")
        let expected_1 = blake2b_hash_64("1");
        let expected_2 = blake2b_hash_64("2");

        let mut gen = OperationIdGenerator::new(None);
        assert_eq!(gen.next_id(), expected_1);
        assert_eq!(gen.next_id(), expected_2);
    }

    #[test]
    fn child_counter_format() {
        // Verify the hash input is "{parent_id}-{counter}" for children.
        let parent = "abc";
        let expected_1 = blake2b_hash_64("abc-1");
        let expected_2 = blake2b_hash_64("abc-2");

        let mut gen = OperationIdGenerator::new(Some(parent.to_string()));
        assert_eq!(gen.next_id(), expected_1);
        assert_eq!(gen.next_id(), expected_2);
    }
}
