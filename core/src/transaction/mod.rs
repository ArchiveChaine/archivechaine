//! Module des transactions pour ArchiveChain

pub mod pool;
pub mod validation;
pub mod types;

pub use types::{Transaction, TransactionType, TransactionInput, TransactionOutput};
pub use pool::TransactionPool;
pub use validation::{TransactionValidator, Validatable};

use crate::error::{TransactionError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_module_basic() {
        // Test basique pour vérifier que le module se compile
        assert!(true);
    }
}