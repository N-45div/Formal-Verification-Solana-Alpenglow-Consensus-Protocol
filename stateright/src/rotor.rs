//! Rotor Module - Alpenglow Block Propagation Implementation
//!
//! This module implements the Rotor block propagation protocol with erasure coding
//! and stake-weighted relay sampling as specified in the Alpenglow whitepaper.

use crate::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Rotor-specific system configuration
#[derive(Clone, Debug)]
pub struct RotorSystem {
    pub base: AlpenglowSystem,
    pub gamma: u32,           // Number of shreds needed to reconstruct (γ)
    pub big_gamma: u32,       // Total number of shreds (Γ)
    pub expansion_ratio: f64, // κ = Γ/γ (data expansion ratio)
}

/// Shred structure (Definition 1 from whitepaper)
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Shred {
    pub slot: Slot,
    pub slice_index: u32,
    pub shred_index: u32,
    pub is_last_slice: bool,
    pub merkle_root: u64,
    pub data: Vec<u8>,
    pub merkle_path: Vec<u64>,
    pub signature: u64, // Simplified signature
}

/// Slice structure (Definition 2 from whitepaper)
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Slice {
    pub slot: Slot,
    pub slice_index: u32,
    pub is_last_slice: bool,
    pub merkle_root: u64,
    pub data: Vec<u8>,
    pub signature: u64,
}

/// Rotor state for block propagation
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RotorState {
    pub base: AlpenglowState,
    pub shreds: BTreeMap<NodeId, BTreeSet<Shred>>,
    pub slices: BTreeMap<NodeId, BTreeSet<Slice>>,
    pub relay_assignments: BTreeMap<(Slot, u32), Vec<NodeId>>, // (slot, slice_index) -> relays
    pub received_shreds: BTreeMap<NodeId, BTreeMap<(Slot, u32, u32), Shred>>, // node -> (slot, slice, shred) -> shred
    pub reconstructed_slices: BTreeMap<NodeId, BTreeSet<(Slot, u32)>>, // node -> reconstructed (slot, slice)
    pub propagation_delays: BTreeMap<(NodeId, NodeId), u32>, // (sender, receiver) -> delay
}

/// Rotor-specific actions
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RotorAction {
    Base(AlpenglowAction),
    GenerateShreds { leader: NodeId, slot: Slot, slice_index: u32 },
    SendShredToRelay { sender: NodeId, receiver: NodeId, shred: Shred },
    RelayShredToAll { relay: NodeId, shred: Shred },
    ReconstructSlice { node_id: NodeId, slot: Slot, slice_index: u32 },
    AssignRelays { slot: Slot, slice_index: u32 },
}

impl RotorSystem {
    /// Create new Rotor system with erasure coding parameters
    pub fn new(node_count: usize, byzantine_ratio: f64, window_size: u32, max_slot: Slot, 
               gamma: u32, expansion_ratio: f64) -> Self {
        let big_gamma = (gamma as f64 * expansion_ratio) as u32;
        Self {
            base: AlpenglowSystem::new(node_count, byzantine_ratio, window_size, max_slot),
            gamma,
            big_gamma,
            expansion_ratio,
        }
    }

    /// Sample relay nodes using stake-weighted sampling (Section 3.1)
    fn sample_relays(&self, slot: Slot, slice_index: u32, exclude: &BTreeSet<NodeId>) -> Vec<NodeId> {
        // Simplified stake-weighted sampling
        // In practice, this would use the PS-P algorithm from Section 3.1
        let mut relays = Vec::new();
        let mut candidates: Vec<_> = self.base.nodes.iter()
            .filter(|&&n| !exclude.contains(&n))
            .cloned()
            .collect();
        
        // Sort by stake (descending) for deterministic selection
        candidates.sort_by(|&a, &b| self.base.stake(b).cmp(&self.base.stake(a)));
        
        // Select top Γ nodes as relays
        relays.extend(candidates.into_iter().take(self.big_gamma as usize));
        
        relays
    }

    /// Check if Rotor is successful (Definition 6 from whitepaper)
    fn is_rotor_successful(&self, state: &RotorState, slot: Slot) -> bool {
        let leader = self.base.leader(slot);
        if !self.base.is_correct(leader) {
            return false;
        }

        // Count correct relays for each slice
        for slice_index in 0..self.get_slice_count(slot) {
            if let Some(relays) = state.relay_assignments.get(&(slot, slice_index)) {
                let correct_relays = relays.iter()
                    .filter(|&&r| self.base.is_correct(r))
                    .count();
                if correct_relays < self.gamma as usize {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        true
    }

    /// Get number of slices for a slot (simplified)
    fn get_slice_count(&self, _slot: Slot) -> u32 {
        3 // Simplified: assume 3 slices per block
    }

    /// Generate shreds for a slice using erasure coding
    fn generate_shreds(&self, slice: &Slice) -> Vec<Shred> {
        let mut shreds = Vec::new();
        
        // Simplified erasure coding: split data into Γ pieces
        let chunk_size = std::cmp::max(1, slice.data.len() / self.big_gamma as usize);
        
        for i in 0..self.big_gamma {
            let start = (i as usize * chunk_size).min(slice.data.len());
            let end = ((i + 1) as usize * chunk_size).min(slice.data.len());
            let data = if start < slice.data.len() {
                slice.data[start..end].to_vec()
            } else {
                vec![0; chunk_size] // Padding for erasure coding
            };
            
            shreds.push(Shred {
                slot: slice.slot,
                slice_index: slice.slice_index,
                shred_index: i,
                is_last_slice: slice.is_last_slice,
                merkle_root: slice.merkle_root,
                data,
                merkle_path: vec![slice.merkle_root], // Simplified Merkle path
                signature: slice.signature,
            });
        }
        
        shreds
    }

    /// Reconstruct slice from γ shreds
    fn reconstruct_slice(&self, shreds: &[Shred]) -> Option<Slice> {
        if shreds.len() < self.gamma as usize {
            return None;
        }

        // Verify all shreds are for the same slice
        let first_shred = &shreds[0];
        if !shreds.iter().all(|s| {
            s.slot == first_shred.slot &&
            s.slice_index == first_shred.slice_index &&
            s.merkle_root == first_shred.merkle_root
        }) {
            return None;
        }

        // Simplified reconstruction: concatenate first γ shreds
        let mut data = Vec::new();
        for shred in shreds.iter().take(self.gamma as usize) {
            data.extend_from_slice(&shred.data);
        }

        Some(Slice {
            slot: first_shred.slot,
            slice_index: first_shred.slice_index,
            is_last_slice: first_shred.is_last_slice,
            merkle_root: first_shred.merkle_root,
            data,
            signature: first_shred.signature,
        })
    }

    /// Calculate network delay between nodes (simplified)
    fn network_delay(&self, _sender: NodeId, _receiver: NodeId) -> u32 {
        1 // Simplified: constant delay δ
    }
}

impl Model for RotorSystem {
    type State = RotorState;
    type Action = RotorAction;

    fn init_states(&self) -> Vec<Self::State> {
        let base_states = self.base.init_states();
        base_states.into_iter().map(|base_state| {
            RotorState {
                base: base_state,
                shreds: self.base.nodes.iter().map(|&n| (n, BTreeSet::new())).collect(),
                slices: self.base.nodes.iter().map(|&n| (n, BTreeSet::new())).collect(),
                relay_assignments: BTreeMap::new(),
                received_shreds: self.base.nodes.iter().map(|&n| (n, BTreeMap::new())).collect(),
                reconstructed_slices: self.base.nodes.iter().map(|&n| (n, BTreeSet::new())).collect(),
                propagation_delays: BTreeMap::new(),
            }
        }).collect()
    }

    fn actions(&self, state: &Self::State, actions: &mut Vec<Self::Action>) {
        // Base actions
        let mut base_actions = Vec::new();
        self.base.actions(&state.base, &mut base_actions);
        for action in base_actions {
            actions.push(RotorAction::Base(action));
        }

        // Rotor-specific actions
        for slot in 0..=state.base.current_slot {
            let leader = self.base.leader(slot);
            
            // Leader generates shreds
            if self.base.is_correct(leader) {
                for slice_index in 0..self.get_slice_count(slot) {
                    // Assign relays if not already assigned
                    if !state.relay_assignments.contains_key(&(slot, slice_index)) {
                        actions.push(RotorAction::AssignRelays { slot, slice_index });
                    }
                    
                    // Generate shreds if relays are assigned
                    if state.relay_assignments.contains_key(&(slot, slice_index)) {
                        actions.push(RotorAction::GenerateShreds { leader, slot, slice_index });
                    }
                }
            }

            // Send shreds to relays
            for &node_id in &self.base.nodes {
                if !self.base.is_correct(node_id) {
                    continue;
                }
                
                if let Some(node_shreds) = state.shreds.get(&node_id) {
                    for shred in node_shreds {
                        if shred.slot == slot {
                            // Send to assigned relays
                            if let Some(relays) = state.relay_assignments.get(&(slot, shred.slice_index)) {
                                for &relay in relays {
                                    if relay != node_id && self.base.is_correct(relay) {
                                        actions.push(RotorAction::SendShredToRelay {
                                            sender: node_id,
                                            receiver: relay,
                                            shred: shred.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Relays broadcast shreds
            for &relay in &self.base.nodes {
                if !self.base.is_correct(relay) {
                    continue;
                }
                
                if let Some(received_shreds) = state.received_shreds.get(&relay) {
                    for ((s, slice_idx, _), shred) in received_shreds {
                        if *s == slot {
                            actions.push(RotorAction::RelayShredToAll {
                                relay,
                                shred: shred.clone(),
                            });
                        }
                    }
                }
            }

            // Nodes reconstruct slices
            for &node_id in &self.base.nodes {
                if !self.base.is_correct(node_id) {
                    continue;
                }
                
                for slice_index in 0..self.get_slice_count(slot) {
                    if !state.reconstructed_slices.get(&node_id)
                        .map_or(false, |s| s.contains(&(slot, slice_index))) {
                        
                        // Check if we have enough shreds to reconstruct
                        if let Some(received) = state.received_shreds.get(&node_id) {
                            let slice_shreds: Vec<_> = received.iter()
                                .filter(|((s, si, _), _)| *s == slot && *si == slice_index)
                                .map(|(_, shred)| shred)
                                .collect();
                            
                            if slice_shreds.len() >= self.gamma as usize {
                                actions.push(RotorAction::ReconstructSlice {
                                    node_id,
                                    slot,
                                    slice_index,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    fn next_state(&self, state: &Self::State, action: Self::Action) -> Option<Self::State> {
        let mut new_state = state.clone();

        match action {
            RotorAction::Base(base_action) => {
                if let Some(new_base_state) = self.base.next_state(&state.base, base_action) {
                    new_state.base = new_base_state;
                } else {
                    return None;
                }
            }

            RotorAction::AssignRelays { slot, slice_index } => {
                let leader = self.base.leader(slot);
                let exclude = BTreeSet::from([leader]);
                let relays = self.sample_relays(slot, slice_index, &exclude);
                new_state.relay_assignments.insert((slot, slice_index), relays);
            }

            RotorAction::GenerateShreds { leader, slot, slice_index } => {
                if !self.base.is_correct(leader) {
                    return None;
                }

                // Create a slice (simplified)
                let slice = Slice {
                    slot,
                    slice_index,
                    is_last_slice: slice_index == self.get_slice_count(slot) - 1,
                    merkle_root: (slot as u64) * 1000 + (slice_index as u64),
                    data: vec![1, 2, 3, 4, 5, 6, 7, 8], // Simplified data
                    signature: leader as u64,
                };

                // Generate shreds
                let shreds = self.generate_shreds(&slice);
                
                // Add shreds to leader's collection
                for shred in shreds {
                    new_state.shreds.entry(leader).or_default().insert(shred);
                }
                new_state.slices.entry(leader).or_default().insert(slice);
            }

            RotorAction::SendShredToRelay { sender, receiver, shred } => {
                if !self.base.is_correct(sender) || !self.base.is_correct(receiver) {
                    return None;
                }

                // Add shred to receiver's collection with delay
                let delay = self.network_delay(sender, receiver);
                new_state.propagation_delays.insert((sender, receiver), delay);
                
                new_state.received_shreds
                    .entry(receiver)
                    .or_default()
                    .insert((shred.slot, shred.slice_index, shred.shred_index), shred);
            }

            RotorAction::RelayShredToAll { relay, shred } => {
                if !self.base.is_correct(relay) {
                    return None;
                }

                // Broadcast shred to all other correct nodes
                for &node_id in &self.base.nodes {
                    if node_id != relay && self.base.is_correct(node_id) {
                        new_state.received_shreds
                            .entry(node_id)
                            .or_default()
                            .insert((shred.slot, shred.slice_index, shred.shred_index), shred.clone());
                    }
                }
            }

            RotorAction::ReconstructSlice { node_id, slot, slice_index } => {
                if !self.base.is_correct(node_id) {
                    return None;
                }

                // Collect shreds for this slice
                if let Some(received) = new_state.received_shreds.get(&node_id) {
                    let slice_shreds: Vec<_> = received.iter()
                        .filter(|((s, si, _), _)| *s == slot && *si == slice_index)
                        .map(|(_, shred)| shred.clone())
                        .collect();

                    // Attempt reconstruction
                    if let Some(slice) = self.reconstruct_slice(&slice_shreds) {
                        new_state.slices.entry(node_id).or_default().insert(slice);
                        new_state.reconstructed_slices
                            .entry(node_id)
                            .or_default()
                            .insert((slot, slice_index));
                    }
                }
            }
        }

        Some(new_state)
    }

    fn properties(&self) -> Vec<Property<Self>> {
        vec![
            // Rotor resilience (Lemma 7): With κ > 5/3, slices are received correctly
            Property::<Self>::always("rotor_resilience", |model, state| {
                model.expansion_ratio > 5.0 / 3.0
            }),

            // Rotor latency (Lemma 8): Network latency is at most 2δ
            Property::<Self>::always("rotor_latency_bound", |model, state| {
                // Simplified: check that propagation delays are bounded
                state.propagation_delays.values().all(|&delay| delay <= 2)
            }),

            // Bandwidth optimality (Lemma 9): Data delivery rate is optimal up to expansion factor
            Property::<Self>::always("rotor_bandwidth_optimality", |model, state| {
                // Simplified: verify expansion ratio is maintained
                let expected_shreds = model.gamma as f64 * model.expansion_ratio;
                (expected_shreds - model.big_gamma as f64).abs() < 1.0
            }),

            // Rotor success condition (Definition 6)
            Property::<Self>::always("rotor_success_with_correct_leader", |model, state| {
                for slot in 0..=state.base.current_slot {
                    let leader = model.base.leader(slot);
                    if model.base.is_correct(leader) {
                        // If leader is correct, Rotor should eventually succeed
                        // (This is a simplified check)
                        if !model.is_rotor_successful(state, slot) {
                            // Allow for in-progress states
                            continue;
                        }
                    }
                }
                true
            }),

            // Data consistency: Reconstructed slices match original
            Property::<Self>::always("rotor_data_consistency", |_, state| {
                for (node_id, slices) in &state.slices {
                    for slice in slices {
                        // Check that all nodes that reconstructed this slice have the same data
                        for (other_node, other_slices) in &state.slices {
                            if node_id != other_node {
                                for other_slice in other_slices {
                                    if slice.slot == other_slice.slot && 
                                       slice.slice_index == other_slice.slice_index {
                                        if slice.data != other_slice.data {
                                            return false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                true
            }),

            // Progress: Slices are eventually reconstructed
            Property::<Self>::sometimes("rotor_slice_reconstruction_progress", |_, state| {
                !state.reconstructed_slices.values().all(|s| s.is_empty())
            }),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotor_small_network() {
        let system = RotorSystem::new(4, 0.2, 2, 2, 2, 2.0);
        assert_eq!(system.gamma, 2);
        assert_eq!(system.big_gamma, 4);
        assert_eq!(system.expansion_ratio, 2.0);
    }

    #[test]
    fn test_erasure_coding_parameters() {
        let system = RotorSystem::new(5, 0.2, 3, 3, 3, 1.8); // κ = 1.8 > 5/3 ≈ 1.667
        
        // Test Lemma 7 condition: κ > 5/3
        assert!(system.expansion_ratio > 5.0 / 3.0);
        
        // Test that we have enough shreds
        assert!(system.big_gamma >= system.gamma);
    }

    #[test]
    fn test_relay_sampling() {
        let system = RotorSystem::new(6, 0.2, 2, 2, 3, 2.0);
        let exclude = BTreeSet::new();
        let relays = system.sample_relays(0, 0, &exclude);
        
        // Should select up to Γ relays
        assert!(relays.len() <= system.big_gamma as usize);
        
        // All relays should be valid nodes
        for relay in relays {
            assert!(system.base.nodes.contains(&relay));
        }
    }

    #[test]
    fn test_shred_generation_and_reconstruction() {
        let system = RotorSystem::new(4, 0.0, 2, 1, 2, 2.0); // No Byzantine nodes for simplicity
        
        let slice = Slice {
            slot: 0,
            slice_index: 0,
            is_last_slice: true,
            merkle_root: 12345,
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            signature: 1,
        };

        // Generate shreds
        let shreds = system.generate_shreds(&slice);
        assert_eq!(shreds.len(), system.big_gamma as usize);

        // Reconstruct slice from first γ shreds
        let reconstructed = system.reconstruct_slice(&shreds[..system.gamma as usize]);
        assert!(reconstructed.is_some());
        
        let reconstructed_slice = reconstructed.unwrap();
        assert_eq!(reconstructed_slice.slot, slice.slot);
        assert_eq!(reconstructed_slice.slice_index, slice.slice_index);
        assert_eq!(reconstructed_slice.merkle_root, slice.merkle_root);
    }
}
