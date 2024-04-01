//! Program instructions
//! A solana program contains a number of instructions.
//! There are 2 instructions in this example:
//!     Init{} initializing some loan information and
//!     Usd2Sol{} transfer usd amount to sol amount.

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum ExampleInstructions {
    Init {},
    Usd2Sol {
        usd_qty: i64,
    },
}