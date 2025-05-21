//! This module provides utility functions to imporove code readability and maintainability.
//!
//!

/// Get the (1 << $idx) value.
macro_rules! bit {
    ($idx:expr) => {
        (1 << $idx)
    };
}
