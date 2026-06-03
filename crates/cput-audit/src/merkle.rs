//! A minimal binary Merkle tree over SHA-256 with inclusion proofs.

use sha2::{Digest, Sha256};

const LEAF_PREFIX: u8 = 0x00;
const NODE_PREFIX: u8 = 0x01;

/// Hash a leaf payload (domain-separated from internal nodes).
#[must_use]
pub fn hash_leaf(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([LEAF_PREFIX]);
    h.update(data);
    h.finalize().into()
}

fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([NODE_PREFIX]);
    h.update(left);
    h.update(right);
    h.finalize().into()
}

/// An inclusion proof for a single leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    /// Index of the leaf within the tree.
    pub index: usize,
    /// Sibling hashes from leaf to root.
    pub siblings: Vec<[u8; 32]>,
}

/// Recompute the root implied by a leaf and its inclusion proof.
#[must_use]
pub fn root_from_proof(leaf: &[u8; 32], proof: &MerkleProof) -> [u8; 32] {
    let mut node = *leaf;
    let mut idx = proof.index;
    for sib in &proof.siblings {
        node = if idx % 2 == 0 {
            hash_node(&node, sib)
        } else {
            hash_node(sib, &node)
        };
        idx /= 2;
    }
    node
}

/// Compute the Merkle root over an ordered list of leaf hashes. Odd levels
/// duplicate the final node (Bitcoin-style). Empty input yields the zero hash.
#[must_use]
pub fn compute_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut level: Vec<[u8; 32]> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };
            next.push(hash_node(&pair[0], right));
        }
        level = next;
    }
    level[0]
}

/// Build an inclusion proof for `index` over the given leaves.
#[must_use]
pub fn build_proof(leaves: &[[u8; 32]], index: usize) -> Option<MerkleProof> {
    if index >= leaves.len() {
        return None;
    }
    let mut siblings = Vec::new();
    let mut level: Vec<[u8; 32]> = leaves.to_vec();
    let mut idx = index;
    while level.len() > 1 {
        let sib_idx = if idx % 2 == 0 {
            (idx + 1).min(level.len() - 1)
        } else {
            idx - 1
        };
        siblings.push(level[sib_idx]);
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };
            next.push(hash_node(&pair[0], right));
        }
        level = next;
        idx /= 2;
    }
    Some(MerkleProof { index, siblings })
}
