#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

/// Merkle tree array based version with floating root point

pub use crate::merkletree::{Hash, MerkleTree, OptionHash};

pub mod hash;
pub mod merkletree;

