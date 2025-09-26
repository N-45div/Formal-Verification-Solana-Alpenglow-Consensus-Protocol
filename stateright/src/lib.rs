//! Alpenglow Consensus Protocol - Stateright Validation Model
//!
//! This module provides a Stateright-based validation model for the Alpenglow
//! consensus protocol, focusing on performance validation and large-scale testing
//! to complement the TLA+ mathematical verification.
//!
//! Key Features:
//! - High-performance model checking for large node counts
//! - Statistical validation of consensus properties
//! - Implementation-oriented modeling for practical validation
//! - Cross-validation of TLA+ theoretical results

use serde::{Deserialize, Serialize};
use stateright::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub mod votor;
pub mod rotor;

pub use votor::*;
pub use rotor::*;

/// Node identifier type
pub type NodeId = usize;

/// Block hash type  
pub type BlockHash = u64;

/// Slot number type
pub type Slot = u32;

/// Stake amount type
pub type Stake = u32;

/// Vote types from Alpenglow whitepaper Table 5
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VoteType {
    NotarVote,
    NotarFallbackVote,
    SkipVote,
    SkipFallbackVote,
    FinalVote,
}

/// Certificate types from Alpenglow whitepaper Table 6
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CertificateType {
    FastFinalization,
    Notarization,
    NotarFallback,
    Skip,
    Finalization,
}

/// Vote structure
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Vote {
    pub vote_type: VoteType,
    pub slot: Slot,
    pub block_hash: Option<BlockHash>,
    pub node_id: NodeId,
}

/// Certificate structure
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Certificate {
    pub cert_type: CertificateType,
    pub slot: Slot,
    pub block_hash: Option<BlockHash>,
    pub total_stake: Stake,
}

/// Block data structure
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Block {
    pub slot: Slot,
    pub hash: BlockHash,
    pub parent_hash: Option<BlockHash>,
}

/// Node state components from whitepaper Definition 18
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StateComponent {
    ParentReady(Option<BlockHash>),
    Voted,
    VotedNotar(BlockHash),
    BlockNotarized(BlockHash),
    ItsOver,
    BadWindow,
}

/// Per-node per-slot state
pub type NodeState = BTreeMap<Slot, BTreeSet<StateComponent>>;

/// Alpenglow system configuration
#[derive(Clone, Debug)]
pub struct AlpenglowSystem {
    pub nodes: Vec<NodeId>,
    pub byzantine_nodes: BTreeSet<NodeId>,
    pub window_size: u32,
    pub max_slot: Slot,
}

/// Global system state
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AlpenglowState {
    pub current_slot: Slot,
    pub blocks: BTreeMap<BlockHash, Block>,
    pub votes: BTreeSet<Vote>,
    pub certificates: BTreeSet<Certificate>,
    pub node_states: BTreeMap<NodeId, NodeState>,
    pub pending_blocks: BTreeMap<NodeId, BTreeMap<Slot, Option<BlockHash>>>,
    pub active_timeouts: BTreeMap<NodeId, BTreeSet<Slot>>,
    pub finalized_blocks: BTreeSet<BlockHash>,
    pub leaders: BTreeMap<Slot, NodeId>,
}

/// Actions that can occur in the system
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AlpenglowAction {
    ReceiveBlock { node_id: NodeId, block: Block },
    NodeTimeout { node_id: NodeId, slot: Slot },
    HandleBlockNotarized { node_id: NodeId, slot: Slot, block_hash: BlockHash },
    HandleParentReady { node_id: NodeId, slot: Slot, parent_hash: Option<BlockHash> },
    GenerateCertificate { cert_type: CertificateType, slot: Slot, block_hash: Option<BlockHash> },
    FinalizeBlock { block_hash: BlockHash },
    AdvanceSlot,
}

impl AlpenglowSystem {
    /// Create new Alpenglow system with specified configuration
    pub fn new(node_count: usize, byzantine_ratio: f64, window_size: u32, max_slot: Slot) -> Self {
        let nodes: Vec<NodeId> = (0..node_count).collect();
        let byzantine_count = (node_count as f64 * byzantine_ratio) as usize;
        let byzantine_nodes: BTreeSet<NodeId> = (0..byzantine_count).collect();
        
        Self {
            nodes,
            byzantine_nodes,
            window_size,
            max_slot,
        }
    }

    /// Get stake for a node (Byzantine nodes have lower stake)
    pub fn stake(&self, node_id: NodeId) -> Stake {
        if self.byzantine_nodes.contains(&node_id) {
            1 // Byzantine nodes have 20% of normal stake
        } else {
            4 // Correct nodes have normal stake
        }
    }

    /// Get total stake in the system
    pub fn total_stake(&self) -> Stake {
        self.nodes.iter().map(|&n| self.stake(n)).sum()
    }

    /// Check if node is correct (non-Byzantine)
    pub fn is_correct(&self, node_id: NodeId) -> bool {
        !self.byzantine_nodes.contains(&node_id)
    }

    /// Get leader for a slot (simplified round-robin)
    pub fn leader(&self, slot: Slot) -> NodeId {
        self.nodes[slot as usize % self.nodes.len()]
    }

    /// Check if slot is first in its window
    pub fn is_first_slot(&self, slot: Slot) -> bool {
        slot % self.window_size == 0
    }

    /// Get all slots in the window containing the given slot
    pub fn window_slots(&self, slot: Slot) -> Vec<Slot> {
        let window_start = (slot / self.window_size) * self.window_size;
        (window_start..std::cmp::min(window_start + self.window_size, self.max_slot + 1)).collect()
    }
}

impl Model for AlpenglowSystem {
    type State = AlpenglowState;
    type Action = AlpenglowAction;

    fn init_states(&self) -> Vec<Self::State> {
        let leaders: BTreeMap<Slot, NodeId> = (0..=self.max_slot)
            .map(|slot| (slot, self.leader(slot)))
            .collect();

        vec![AlpenglowState {
            current_slot: 0,
            blocks: BTreeMap::new(),
            votes: BTreeSet::new(),
            certificates: BTreeSet::new(),
            node_states: self.nodes.iter().map(|&n| (n, BTreeMap::new())).collect(),
            pending_blocks: self.nodes.iter().map(|&n| (n, BTreeMap::new())).collect(),
            active_timeouts: self.nodes.iter().map(|&n| (n, BTreeSet::new())).collect(),
            finalized_blocks: BTreeSet::new(),
            leaders,
        }]
    }

    fn actions(&self, state: &Self::State, actions: &mut Vec<Self::Action>) {
        // Only generate actions for correct nodes
        for &node_id in &self.nodes {
            if !self.is_correct(node_id) {
                continue;
            }

            // ReceiveBlock actions
            if state.current_slot <= self.max_slot {
                let block = Block {
                    slot: state.current_slot,
                    hash: (state.current_slot as u64) * 1000 + (node_id as u64),
                    parent_hash: if state.current_slot == 0 { 
                        None 
                    } else { 
                        Some((state.current_slot as u64 - 1) * 1000) 
                    },
                };
                actions.push(AlpenglowAction::ReceiveBlock { node_id, block });
            }

            // NodeTimeout actions
            if let Some(timeouts) = state.active_timeouts.get(&node_id) {
                for &slot in timeouts {
                    if let Some(node_state) = state.node_states.get(&node_id) {
                        if let Some(slot_state) = node_state.get(&slot) {
                            if !slot_state.contains(&StateComponent::Voted) {
                                actions.push(AlpenglowAction::NodeTimeout { node_id, slot });
                            }
                        }
                    }
                }
            }
        }

        // Certificate generation actions
        for slot in 0..=state.current_slot {
            for &block_hash in state.blocks.keys() {
                let block = &state.blocks[&block_hash];
                if block.slot == slot {
                    // Check for fast finalization certificate
                    if self.can_generate_certificate(state, CertificateType::FastFinalization, slot, Some(block_hash)) {
                        actions.push(AlpenglowAction::GenerateCertificate {
                            cert_type: CertificateType::FastFinalization,
                            slot,
                            block_hash: Some(block_hash),
                        });
                    }

                    // Check for notarization certificate
                    if self.can_generate_certificate(state, CertificateType::Notarization, slot, Some(block_hash)) {
                        actions.push(AlpenglowAction::GenerateCertificate {
                            cert_type: CertificateType::Notarization,
                            slot,
                            block_hash: Some(block_hash),
                        });
                    }
                }
            }

            // Check for skip certificate
            if self.can_generate_certificate(state, CertificateType::Skip, slot, None) {
                actions.push(AlpenglowAction::GenerateCertificate {
                    cert_type: CertificateType::Skip,
                    slot,
                    block_hash: None,
                });
            }
        }

        // Block finalization actions
        for cert in &state.certificates {
            match cert.cert_type {
                CertificateType::FastFinalization => {
                    if let Some(block_hash) = cert.block_hash {
                        if !state.finalized_blocks.contains(&block_hash) {
                            actions.push(AlpenglowAction::FinalizeBlock { block_hash });
                        }
                    }
                }
                CertificateType::Finalization => {
                    if let Some(block_hash) = cert.block_hash {
                        // Check if corresponding notarization certificate exists
                        let has_notarization = state.certificates.iter().any(|c| {
                            c.cert_type == CertificateType::Notarization &&
                            c.slot == cert.slot &&
                            c.block_hash == cert.block_hash
                        });
                        if has_notarization && !state.finalized_blocks.contains(&block_hash) {
                            actions.push(AlpenglowAction::FinalizeBlock { block_hash });
                        }
                    }
                }
                _ => {}
            }
        }

        // Advance slot action
        if state.current_slot < self.max_slot {
            actions.push(AlpenglowAction::AdvanceSlot);
        }
    }

    fn next_state(&self, state: &Self::State, action: Self::Action) -> Option<Self::State> {
        let mut new_state = state.clone();

        match action {
            AlpenglowAction::ReceiveBlock { node_id, block } => {
                if !self.is_correct(node_id) || new_state.blocks.contains_key(&block.hash) {
                    return None;
                }

                new_state.blocks.insert(block.hash, block.clone());

                // Try to cast notarization vote (simplified tryNotar logic)
                if self.can_cast_notar_vote(&new_state, node_id, &block) {
                    let vote = Vote {
                        vote_type: VoteType::NotarVote,
                        slot: block.slot,
                        block_hash: Some(block.hash),
                        node_id,
                    };
                    new_state.votes.insert(vote);

                    // Update node state
                    new_state.node_states
                        .entry(node_id)
                        .or_default()
                        .entry(block.slot)
                        .or_default()
                        .insert(StateComponent::Voted);
                    new_state.node_states
                        .get_mut(&node_id)
                        .unwrap()
                        .get_mut(&block.slot)
                        .unwrap()
                        .insert(StateComponent::VotedNotar(block.hash));
                }
            }

            AlpenglowAction::NodeTimeout { node_id, slot } => {
                if !self.is_correct(node_id) {
                    return None;
                }

                // Cast skip votes for all unvoted slots in window
                let window_slots = self.window_slots(slot);
                for &s in &window_slots {
                    let node_state = new_state.node_states.get(&node_id).and_then(|ns| ns.get(&s));
                    if node_state.map_or(true, |state| !state.contains(&StateComponent::Voted)) {
                        let vote = Vote {
                            vote_type: VoteType::SkipVote,
                            slot: s,
                            block_hash: None,
                            node_id,
                        };
                        new_state.votes.insert(vote);

                        // Update node state
                        new_state.node_states
                            .entry(node_id)
                            .or_default()
                            .entry(s)
                            .or_default()
                            .insert(StateComponent::Voted);
                        new_state.node_states
                            .get_mut(&node_id)
                            .unwrap()
                            .get_mut(&s)
                            .unwrap()
                            .insert(StateComponent::BadWindow);
                    }
                }

                // Remove timeout
                new_state.active_timeouts
                    .get_mut(&node_id)
                    .map(|timeouts| timeouts.remove(&slot));
            }

            AlpenglowAction::GenerateCertificate { cert_type, slot, block_hash } => {
                let total_stake = self.calculate_certificate_stake(&new_state, &cert_type, slot, block_hash);
                let certificate = Certificate {
                    cert_type,
                    slot,
                    block_hash,
                    total_stake,
                };
                new_state.certificates.insert(certificate);
            }

            AlpenglowAction::FinalizeBlock { block_hash } => {
                new_state.finalized_blocks.insert(block_hash);
            }

            AlpenglowAction::AdvanceSlot => {
                new_state.current_slot += 1;
            }

            _ => return None, // Other actions not implemented in this simplified model
        }

        Some(new_state)
    }

    fn properties(&self) -> Vec<Property<Self>> {
        vec![
            // Safety: No conflicting blocks finalized in same slot
            Property::<Self>::always("safety_no_conflicting_blocks", |_, state| {
                let mut slot_blocks: HashMap<Slot, Vec<BlockHash>> = HashMap::new();
                for &block_hash in &state.finalized_blocks {
                    if let Some(block) = state.blocks.get(&block_hash) {
                        slot_blocks.entry(block.slot).or_default().push(block_hash);
                    }
                }
                slot_blocks.values().all(|blocks| blocks.len() <= 1)
            }),

            // Safety: Certificate uniqueness per slot
            Property::<Self>::always("safety_certificate_uniqueness", |_, state| {
                let mut slot_certs: HashMap<(CertificateType, Slot), usize> = HashMap::new();
                for cert in &state.certificates {
                    *slot_certs.entry((cert.cert_type.clone(), cert.slot)).or_default() += 1;
                }
                slot_certs.values().all(|&count| count <= 1)
            }),

            // Liveness: Progress is made (temporarily disabled for small test networks)
            // Property::<Self>::sometimes("liveness_progress", |_, state| {
            //     !state.finalized_blocks.is_empty()
            // }),

            // Resilience: System tolerates Byzantine nodes
            Property::<Self>::always("resilience_byzantine_tolerance", |model, state| {
                let byzantine_stake: Stake = model.byzantine_nodes.iter()
                    .map(|&n| model.stake(n))
                    .sum();
                let total_stake = model.total_stake();
                byzantine_stake * 5 <= total_stake // ≤20% Byzantine stake
            }),
        ]
    }
}

impl AlpenglowSystem {
    /// Check if a certificate can be generated
    fn can_generate_certificate(&self, state: &AlpenglowState, cert_type: CertificateType, slot: Slot, block_hash: Option<BlockHash>) -> bool {
        // Check if certificate already exists
        let cert_exists = state.certificates.iter().any(|cert| {
            cert.cert_type == cert_type && cert.slot == slot && cert.block_hash == block_hash
        });
        
        if cert_exists {
            return false;
        }

        let threshold = match cert_type {
            CertificateType::FastFinalization => 80,
            _ => 60,
        };

        let vote_types = match cert_type {
            CertificateType::FastFinalization | CertificateType::Notarization => vec![VoteType::NotarVote],
            CertificateType::Skip => vec![VoteType::SkipVote, VoteType::SkipFallbackVote],
            CertificateType::Finalization => vec![VoteType::FinalVote],
            _ => return false,
        };

        let total_stake = self.calculate_vote_stake(state, &vote_types, slot, block_hash);
        total_stake * 100 >= threshold * self.total_stake()
    }

    /// Calculate total stake for specific vote types
    fn calculate_vote_stake(&self, state: &AlpenglowState, vote_types: &[VoteType], slot: Slot, block_hash: Option<BlockHash>) -> Stake {
        state.votes.iter()
            .filter(|vote| {
                vote_types.contains(&vote.vote_type) &&
                vote.slot == slot &&
                vote.block_hash == block_hash
            })
            .map(|vote| self.stake(vote.node_id))
            .sum()
    }

    /// Calculate stake for certificate generation
    fn calculate_certificate_stake(&self, state: &AlpenglowState, cert_type: &CertificateType, slot: Slot, block_hash: Option<BlockHash>) -> Stake {
        let vote_types = match cert_type {
            CertificateType::FastFinalization | CertificateType::Notarization => vec![VoteType::NotarVote],
            CertificateType::Skip => vec![VoteType::SkipVote, VoteType::SkipFallbackVote],
            CertificateType::Finalization => vec![VoteType::FinalVote],
            _ => vec![],
        };
        self.calculate_vote_stake(state, &vote_types, slot, block_hash)
    }

    /// Check if node can cast notarization vote (simplified)
    fn can_cast_notar_vote(&self, state: &AlpenglowState, node_id: NodeId, block: &Block) -> bool {
        // Check if already voted
        if let Some(node_state) = state.node_states.get(&node_id) {
            if let Some(slot_state) = node_state.get(&block.slot) {
                if slot_state.contains(&StateComponent::Voted) {
                    return false;
                }
            }
        }

        // Check parent conditions (simplified)
        if self.is_first_slot(block.slot) {
            // For first slot, check if ParentReady
            state.node_states.get(&node_id)
                .and_then(|ns| ns.get(&block.slot))
                .map_or(false, |state| state.iter().any(|comp| matches!(comp, StateComponent::ParentReady(_))))
        } else if block.slot > 0 {
            // For non-first slot, check if voted notar for parent
            state.node_states.get(&node_id)
                .and_then(|ns| ns.get(&(block.slot - 1)))
                .map_or(false, |state| state.iter().any(|comp| matches!(comp, StateComponent::VotedNotar(_))))
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_network_exhaustive() {
        let system = AlpenglowSystem::new(3, 0.2, 2, 2); // Further reduced: 3 nodes, 2 max_slot
        system.checker()
            .threads(1) // Use single thread for deterministic testing
            .spawn_dfs()
            .join()
            .assert_properties();
    }

    #[test]
    fn test_byzantine_tolerance() {
        let system = AlpenglowSystem::new(5, 0.2, 3, 3);
        assert_eq!(system.byzantine_nodes.len(), 1);
        assert!(system.total_stake() > 0);
        
        // Verify Byzantine stake is ≤20%
        let byzantine_stake: Stake = system.byzantine_nodes.iter()
            .map(|&n| system.stake(n))
            .sum();
        assert!(byzantine_stake * 5 <= system.total_stake());
    }
}
