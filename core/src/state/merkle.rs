//! Implémentation d'arbre de Merkle pour ArchiveChain
//! 
//! Fournit un arbre de Merkle efficace pour maintenir l'intégrité des données

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::crypto::{Hash, HashAlgorithm, compute_hash, compute_combined_hash};
use crate::error::{StateError, Result};

/// Nœud d'un arbre de Merkle
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MerkleNode {
    /// Nœud feuille contenant des données
    Leaf {
        /// Hash des données
        hash: Hash,
        /// Données optionnelles (peuvent être stockées séparément)
        data: Option<Vec<u8>>,
    },
    /// Nœud interne avec deux enfants
    Internal {
        /// Hash calculé à partir des enfants
        hash: Hash,
        /// Index du nœud gauche
        left: usize,
        /// Index du nœud droit
        right: usize,
    },
}

impl MerkleNode {
    /// Obtient le hash du nœud
    pub fn hash(&self) -> &Hash {
        match self {
            MerkleNode::Leaf { hash, .. } => hash,
            MerkleNode::Internal { hash, .. } => hash,
        }
    }

    /// Vérifie si le nœud est une feuille
    pub fn is_leaf(&self) -> bool {
        matches!(self, MerkleNode::Leaf { .. })
    }

    /// Obtient les données si c'est une feuille
    pub fn data(&self) -> Option<&[u8]> {
        match self {
            MerkleNode::Leaf { data, .. } => data.as_deref(),
            MerkleNode::Internal { .. } => None,
        }
    }
}

/// Preuve de Merkle pour vérifier qu'un élément fait partie de l'arbre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Hash de l'élément à prouver
    pub leaf_hash: Hash,
    /// Chemin de preuves (hash et position - true pour droite, false pour gauche)
    pub path: Vec<(Hash, bool)>,
    /// Hash de la racine
    pub root_hash: Hash,
}

impl MerkleProof {
    /// Vérifie la validité de la preuve
    pub fn verify(&self, algorithm: HashAlgorithm) -> bool {
        let mut current_hash = self.leaf_hash.clone();
        
        for (sibling_hash, is_right) in &self.path {
            current_hash = if *is_right {
                // Le sibling est à droite, donc current_hash est à gauche
                compute_combined_hash(&[current_hash.as_bytes(), sibling_hash.as_bytes()], algorithm)
            } else {
                // Le sibling est à gauche, donc current_hash est à droite
                compute_combined_hash(&[sibling_hash.as_bytes(), current_hash.as_bytes()], algorithm)
            };
        }
        
        current_hash == self.root_hash
    }
}

/// Arbre de Merkle avec stockage efficace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Nœuds de l'arbre stockés dans un vecteur
    nodes: Vec<MerkleNode>,
    /// Index de la racine
    root_index: Option<usize>,
    /// Algorithme de hachage utilisé
    algorithm: HashAlgorithm,
    /// Index des feuilles pour un accès rapide
    leaf_indices: HashMap<Hash, usize>,
}

impl MerkleTree {
    /// Crée un nouvel arbre de Merkle vide
    pub fn new(algorithm: HashAlgorithm) -> Self {
        Self {
            nodes: Vec::new(),
            root_index: None,
            algorithm,
            leaf_indices: HashMap::new(),
        }
    }

    /// Construit un arbre de Merkle à partir de données
    pub fn from_data(data_items: Vec<Vec<u8>>, algorithm: HashAlgorithm) -> Self {
        let mut tree = Self::new(algorithm);
        
        if data_items.is_empty() {
            return tree;
        }
        
        // Crée les feuilles
        let mut current_level: Vec<usize> = Vec::new();
        for data in data_items {
            let hash = compute_hash(&data, algorithm);
            let leaf = MerkleNode::Leaf {
                hash: hash.clone(),
                data: Some(data),
            };
            let index = tree.nodes.len();
            tree.leaf_indices.insert(hash, index);
            tree.nodes.push(leaf);
            current_level.push(index);
        }
        
        // Construit l'arbre niveau par niveau
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            // Traite les paires de nœuds
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    // Paire complète
                    let left_idx = chunk[0];
                    let right_idx = chunk[1];
                    let left_hash = tree.nodes[left_idx].hash();
                    let right_hash = tree.nodes[right_idx].hash();
                    
                    let combined_hash = compute_combined_hash(
                        &[left_hash.as_bytes(), right_hash.as_bytes()],
                        algorithm
                    );
                    
                    let internal = MerkleNode::Internal {
                        hash: combined_hash,
                        left: left_idx,
                        right: right_idx,
                    };
                    
                    let index = tree.nodes.len();
                    tree.nodes.push(internal);
                    next_level.push(index);
                } else {
                    // Nœud orphelin - promouvoir au niveau suivant
                    next_level.push(chunk[0]);
                }
            }
            
            current_level = next_level;
        }
        
        // Définit la racine
        if !current_level.is_empty() {
            tree.root_index = Some(current_level[0]);
        }
        
        tree
    }

    /// Construit un arbre à partir de hashs existants
    pub fn from_hashes(hashes: Vec<Hash>, algorithm: HashAlgorithm) -> Self {
        let mut tree = Self::new(algorithm);
        
        if hashes.is_empty() {
            return tree;
        }
        
        // Crée les feuilles sans données
        let mut current_level: Vec<usize> = Vec::new();
        for hash in hashes {
            let leaf = MerkleNode::Leaf {
                hash: hash.clone(),
                data: None,
            };
            let index = tree.nodes.len();
            tree.leaf_indices.insert(hash, index);
            tree.nodes.push(leaf);
            current_level.push(index);
        }
        
        // Construit l'arbre comme précédemment
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let left_idx = chunk[0];
                    let right_idx = chunk[1];
                    let left_hash = tree.nodes[left_idx].hash();
                    let right_hash = tree.nodes[right_idx].hash();
                    
                    let combined_hash = compute_combined_hash(
                        &[left_hash.as_bytes(), right_hash.as_bytes()],
                        algorithm
                    );
                    
                    let internal = MerkleNode::Internal {
                        hash: combined_hash,
                        left: left_idx,
                        right: right_idx,
                    };
                    
                    let index = tree.nodes.len();
                    tree.nodes.push(internal);
                    next_level.push(index);
                } else {
                    next_level.push(chunk[0]);
                }
            }
            
            current_level = next_level;
        }
        
        if !current_level.is_empty() {
            tree.root_index = Some(current_level[0]);
        }
        
        tree
    }

    /// Obtient le hash de la racine
    pub fn root_hash(&self) -> Option<&Hash> {
        self.root_index.map(|idx| self.nodes[idx].hash())
    }

    /// Génère une preuve de Merkle pour un hash donné
    pub fn generate_proof(&self, target_hash: &Hash) -> Result<MerkleProof> {
        let leaf_index = self.leaf_indices.get(target_hash)
            .ok_or(StateError::MerkleNodeNotFound)?;
        
        let root_hash = self.root_hash()
            .ok_or(StateError::InvalidMerkleRoot)?
            .clone();
        
        let mut path = Vec::new();
        let mut current_index = *leaf_index;
        
        // Remonte l'arbre jusqu'à la racine
        for node in &self.nodes {
            if let MerkleNode::Internal { left, right, .. } = node {
                if *left == current_index {
                    // Le nœud courant est à gauche, ajoute le sibling droit
                    let sibling_hash = self.nodes[*right].hash().clone();
                    path.push((sibling_hash, true)); // true = sibling à droite
                    
                    // Trouve l'index du nœud parent
                    if let Some(parent_idx) = self.find_parent_index(current_index) {
                        current_index = parent_idx;
                    } else {
                        break;
                    }
                } else if *right == current_index {
                    // Le nœud courant est à droite, ajoute le sibling gauche
                    let sibling_hash = self.nodes[*left].hash().clone();
                    path.push((sibling_hash, false)); // false = sibling à gauche
                    
                    if let Some(parent_idx) = self.find_parent_index(current_index) {
                        current_index = parent_idx;
                    } else {
                        break;
                    }
                }
            }
        }
        
        Ok(MerkleProof {
            leaf_hash: target_hash.clone(),
            path,
            root_hash,
        })
    }

    /// Trouve l'index du parent d'un nœud
    fn find_parent_index(&self, child_index: usize) -> Option<usize> {
        for (i, node) in self.nodes.iter().enumerate() {
            if let MerkleNode::Internal { left, right, .. } = node {
                if *left == child_index || *right == child_index {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Vérifie si un hash est présent dans l'arbre
    pub fn contains(&self, hash: &Hash) -> bool {
        self.leaf_indices.contains_key(hash)
    }

    /// Obtient le nombre de feuilles
    pub fn leaf_count(&self) -> usize {
        self.leaf_indices.len()
    }

    /// Obtient le nombre total de nœuds
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Vérifie l'intégrité de l'arbre
    pub fn verify_integrity(&self) -> bool {
        if let Some(root_idx) = self.root_index {
            self.verify_node_integrity(root_idx)
        } else {
            self.nodes.is_empty()
        }
    }

    /// Vérifie l'intégrité d'un nœud récursivement
    fn verify_node_integrity(&self, node_index: usize) -> bool {
        if node_index >= self.nodes.len() {
            return false;
        }
        
        match &self.nodes[node_index] {
            MerkleNode::Leaf { .. } => true,
            MerkleNode::Internal { hash, left, right } => {
                if *left >= self.nodes.len() || *right >= self.nodes.len() {
                    return false;
                }
                
                let left_hash = self.nodes[*left].hash();
                let right_hash = self.nodes[*right].hash();
                let expected_hash = compute_combined_hash(
                    &[left_hash.as_bytes(), right_hash.as_bytes()],
                    self.algorithm
                );
                
                *hash == expected_hash &&
                self.verify_node_integrity(*left) &&
                self.verify_node_integrity(*right)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::compute_blake3;

    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::new(HashAlgorithm::Blake3);
        assert!(tree.root_hash().is_none());
        assert_eq!(tree.leaf_count(), 0);
        assert!(tree.verify_integrity());
    }

    #[test]
    fn test_single_leaf() {
        let data = vec![b"single leaf".to_vec()];
        let tree = MerkleTree::from_data(data, HashAlgorithm::Blake3);
        
        assert!(tree.root_hash().is_some());
        assert_eq!(tree.leaf_count(), 1);
        assert!(tree.verify_integrity());
    }

    #[test]
    fn test_multiple_leaves() {
        let data = vec![
            b"leaf 1".to_vec(),
            b"leaf 2".to_vec(),
            b"leaf 3".to_vec(),
            b"leaf 4".to_vec(),
        ];
        let tree = MerkleTree::from_data(data, HashAlgorithm::Blake3);
        
        assert!(tree.root_hash().is_some());
        assert_eq!(tree.leaf_count(), 4);
        assert!(tree.verify_integrity());
    }

    #[test]
    fn test_odd_number_leaves() {
        let data = vec![
            b"leaf 1".to_vec(),
            b"leaf 2".to_vec(),
            b"leaf 3".to_vec(),
        ];
        let tree = MerkleTree::from_data(data, HashAlgorithm::Blake3);
        
        assert!(tree.root_hash().is_some());
        assert_eq!(tree.leaf_count(), 3);
        assert!(tree.verify_integrity());
    }

    #[test]
    fn test_merkle_proof_generation_and_verification() {
        let data = vec![
            b"data 1".to_vec(),
            b"data 2".to_vec(),
            b"data 3".to_vec(),
            b"data 4".to_vec(),
        ];
        let tree = MerkleTree::from_data(data.clone(), HashAlgorithm::Blake3);
        
        // Génère et vérifie une preuve pour chaque élément
        for item in &data {
            let target_hash = compute_blake3(item);
            assert!(tree.contains(&target_hash));
            
            let proof = tree.generate_proof(&target_hash).unwrap();
            assert!(proof.verify(HashAlgorithm::Blake3));
            assert_eq!(proof.leaf_hash, target_hash);
            assert_eq!(proof.root_hash, *tree.root_hash().unwrap());
        }
    }

    #[test]
    fn test_from_hashes() {
        let hashes = vec![
            compute_blake3(b"hash 1"),
            compute_blake3(b"hash 2"),
            compute_blake3(b"hash 3"),
        ];
        
        let tree = MerkleTree::from_hashes(hashes.clone(), HashAlgorithm::Blake3);
        
        assert_eq!(tree.leaf_count(), 3);
        assert!(tree.verify_integrity());
        
        for hash in &hashes {
            assert!(tree.contains(hash));
        }
    }

    #[test]
    fn test_invalid_proof() {
        let data = vec![b"data 1".to_vec(), b"data 2".to_vec()];
        let tree = MerkleTree::from_data(data, HashAlgorithm::Blake3);
        
        let non_existent_hash = compute_blake3(b"non existent");
        let result = tree.generate_proof(&non_existent_hash);
        assert!(result.is_err());
    }
}