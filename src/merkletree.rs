use anyhow::{bail, Result};
use hex;
use log::{debug, error, info, Level, log_enabled};
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

use crate::hash::Hasher;

pub type Hash = [u8; MerkleTree::HASH_SIZE_BYTES];
pub type OptionHash = Option<Hash>;

pub struct MerkleTree {
    hasher: Box<dyn Hasher + 'static>,
    root: u32,
    zero_index: u32,
    current_add_position: usize,
    nodes: Box<[OptionHash]>,
    default_hash: Hash,
    max_size: u32,
}


impl MerkleTree {
    pub const HASH_SIZE_BYTES: usize = 32;

    pub fn new(levels: u32, hasher: impl Hasher + 'static) -> Self {
        if levels < 1 && levels > 27 {
            panic!("Not acceptable tree size {}. Consider range [1-28]", levels);
        }

        let nodes_size = (1 << levels) - 1;

        info!("Creating merkle tree with size {}", nodes_size);

        let index = ((nodes_size - 1) / 2) as u32;
        let default_hash = hasher.generate_hash(&[0u8; MerkleTree::HASH_SIZE_BYTES]);

        MerkleTree {
            hasher: Box::new(hasher),
            root: index,
            zero_index: index,
            current_add_position: index as usize,
            nodes: vec![Option::None::<[u8; Self::HASH_SIZE_BYTES]>; nodes_size].into_boxed_slice(),
            default_hash,
            max_size: 1 << levels - 1,
        }
    }

    pub fn capacity(&self) -> u32 {
        self.max_size
    }

    pub fn hash_of(&self, index: usize) -> OptionHash {
        self.nodes[index]
    }

    /// returns MT index of added value
    pub fn add(&mut self, value: Hash) -> u32 {
        if !(self.max_size > self.size()) {
            panic!("We full")
        }

        if self.nodes[self.current_add_position].is_some() {
            panic!("Replacing not allowed for 'add' command");
        }

        if log_enabled!(Level::Debug) {
            debug!("Adding {} to i[{}]", Self::to_hex(&value[..3]), self.current_add_position);
        }

        self.nodes[self.current_add_position] = Some(value);
        self.current_add_position += 1;

        let current_lvl = ((self.size() + 1) as f64).log2() as u32;
        self.root = (1 << (self.tree_lvl() - current_lvl)) - 1;

        let node = self.current_add_position as u32 - 1;
        self.update_branch(node);
        node - self.zero_index
    }

    pub fn update(&mut self, index: u32, value: Hash) -> Result<Hash> {
        let index = (index + self.zero_index) as usize;

        if index >= self.current_add_position || index < self.zero_index as usize || self.nodes[index].is_none() {
            bail!(MerkleTreeError::UpdateIndexError)
        }

        let old_hash = self.nodes[index];
        self.nodes[index] = Some(value);

        self.update_branch(index as u32);

        if log_enabled!(Level::Debug) {
            debug!("Updating i[{}]. old: [{}]. new: [{}]",
                   index, Self::to_hex(&old_hash.unwrap()[..3]), Self::to_hex(&self.nodes[index].unwrap()[..3]));
        }

        Ok(old_hash.unwrap())
    }

    fn update_branch(&mut self, mut node: u32) {
        while let Some(parent) = Self::parent(node) {
            let siblings = Self::child_nodes(parent);

            let left = self.nodes[siblings.0 as usize].unwrap_or(self.default_hash);
            let right = self.nodes[siblings.1 as usize].unwrap_or(self.default_hash);
            self.nodes[parent as usize] = Some(self.hasher.concat_hash(&left, &right));
            node = parent;

            if parent == self.root {
                break;
            }
        }
    }

    pub fn tree_lvl(&self) -> u32 {
        let nodes_size = self.nodes.len() as f64;
        nodes_size.log2() as u32
    }

    pub fn size(&self) -> u32 {
        (self.current_add_position - self.zero_index as usize) as u32
    }

    pub fn generate_hash(&self, data: &[u8]) -> Hash {
        self.hasher.generate_hash(data)
    }
}

impl MerkleTree {
    fn left_child(of: u32) -> u32 {
        2 * of + 1
    }

    fn right_child(of: u32) -> u32 {
        2 * of + 2
    }

    fn child_nodes(parent: u32) -> (u32, u32) {
        (Self::left_child(parent), Self::right_child(parent))
    }

    fn parent(of: u32) -> Option<u32> {
        if of == 0 {
            None
        } else {
            Some((of - 1) / 2)
        }
    }

    fn hash_as_hex_with_prefix(hash: &Hash) -> String {
        let mut h = Self::hash_as_hex(hash);
        h.insert_str(0, "0x");
        h
    }

    fn hash_as_hex(hash: &Hash) -> String {
        hex::encode(hash)
    }

    fn to_hex(hash: &[u8]) -> String {
        hex::encode(hash)
    }
}

impl Display for MerkleTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Length: {}, Capacity: {}, Root: {}, Size: {}, Next: {}",
               self.nodes.len(), self.max_size, self.root, self.size(), self.current_add_position)
    }
}

impl Debug for MerkleTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut l = f.debug_list();
        for n in self.nodes.iter() {
            match n {
                Some(r) => l.entry(&Self::to_hex(&r[..3])),
                None => l.entry(&"None")
            };
        }

        l.finish()
    }
}

#[derive(Error, Debug)]
enum MerkleTreeError {
    #[error("Wrong index")]
    UpdateIndexError,

    #[error("Trying to update empty node")]
    UpdateEmptyError,

    #[error("Empty hash value not allowed")]
    UpdateEmptyInputError,
}

#[cfg(test)]
mod tests {
    use env_logger::{Builder, Env};
    use libc::{c_char, c_void};
    use rand::prelude::SliceRandom;
    use rand::Rng;
    use std::ptr::{null, null_mut};

    use crate::ALLOC;
    use crate::hash::ShaHasher;

    use super::*;

    #[global_allocator]
    pub static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

    extern "C" fn write_cb(_: *mut c_void, message: *const c_char) {
        print!("{}", String::from_utf8_lossy(unsafe {
            std::ffi::CStr::from_ptr(message as *const i8).to_bytes()
        }));
    }

    #[test]
    fn merkle_tree() {
        Builder::from_env(Env::default().default_filter_or("debug")).init();

        // memory_usage();

        let levels = 3;
        let mut tree = MerkleTree::new(levels, ShaHasher::default());
        info!("empty: {}", tree);
        // memory_usage();
        let hash_size = std::mem::size_of::<Hash>();
        println!("Hash size: {}mb", hash_size * 1000_000 / 1024 / 1024);

        unsafe {
            let ptr: *const c_void = &tree as *const MerkleTree as *const c_void;
            let size = jemalloc_sys::malloc_usable_size(ptr);
            println!("Size is: {}", size)
        }

        let mut k = 0;
        while k < 1 << levels - 1 {
            let hash = tree.generate_hash("hello".as_bytes());
            tree.add(hash);
            info!("{:?}", tree);

            let hash = tree.generate_hash("12345".as_bytes());
            info!("update: {}", MerkleTree::to_hex(&hash[..3]));

            match tree.update(k, hash) {
                Ok(t) => info!("Updated"),
                Err(e) => panic!("Failed to update tree.\n {}", e),
            }

            info!("{:?}\n", tree);
            k += 1;
        }
        // memory_usage();

        unsafe {
            let ptr: *const c_void = &tree as *const MerkleTree as *const c_void;
            let size = jemalloc_sys::malloc_usable_size(ptr);
            println!("Size is: {}", size)
        }

        info!("final: {}", tree);
    }

    fn memory_usage() {
        unsafe { jemalloc_sys::malloc_stats_print(Some(write_cb), null_mut(), null()) };
    }

    pub trait HeapSizeOf {
        fn size_of_including_self(&self) -> usize;
        fn size_of_excluding_self(&self) -> usize;
    }

    impl<T: HeapSizeOf> HeapSizeOf for Box<T> {
        fn size_of_including_self(&self) -> usize {
            unimplemented!()
        }

        fn size_of_excluding_self(&self) -> usize {
            // heap_size_of(&**self as *const T as *const c_void) + (**self).size_of_excluding_self()
            0
        }
    }
}
