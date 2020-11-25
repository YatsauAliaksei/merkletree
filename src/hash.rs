use tiny_keccak::Sha3;

use crate::{Hash, MerkleTree};

pub trait Hasher {
    fn concat_hash(&self, left: &[u8], right: &[u8]) -> Hash;

    fn generate_hash(&self, data: &[u8]) -> Hash;
}

pub struct ShaHasher {}

impl ShaHasher {
    const fn zero() -> Hash {
        [0; MerkleTree::HASH_SIZE_BYTES]
    }
}

impl Default for ShaHasher {
    fn default() -> Self {
        ShaHasher {}
    }
}

impl Hasher for ShaHasher {
    fn concat_hash(&self, left: &[u8], right: &[u8]) -> Hash {
        use tiny_keccak::Hasher;

        let mut sha = Sha3::v256();
        sha.update(left);
        sha.update(right);
        let mut hash = Self::zero();
        sha.finalize(&mut hash[..]);
        hash
    }

    fn generate_hash(&self, data: &[u8]) -> Hash {
        use tiny_keccak::Hasher;

        let mut sha = Sha3::v256();
        sha.update(data);
        let mut hash = Self::zero();
        sha.finalize(&mut hash[..]);
        hash
    }
}

