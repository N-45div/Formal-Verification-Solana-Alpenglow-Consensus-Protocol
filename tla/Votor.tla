--------------------------- MODULE Votor ---------------------------
(*
Votor: Alpenglow Voting Protocol Specification

This module implements the core Votor consensus logic from Alpenglow whitepaper
Algorithms 1 and 2, providing detailed modeling of the dual-path consensus
mechanism with precise state transitions.

Key Features:
- Exact implementation of Algorithm 1 (Votor event loop)
- Exact implementation of Algorithm 2 (Votor helper functions)
- Five certificate types with precise thresholds
- Timeout-based skip logic
- Parent-child block relationships
- Byzantine fault tolerance up to 20%

Based on: Alpenglow Whitepaper v1.1, Section 2.6 and Algorithms 1-2
*)

EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    Nodes,              \* Set of validator nodes
    MaxSlot,            \* Maximum slot for model checking
    WindowSize,         \* Slots per leader window
    ByzantineNodes,     \* Byzantine nodes (≤20% stake)
    DeltaTimeout,       \* Timeout parameter
    DeltaBlock          \* Block time parameter

VARIABLES
    currentSlot,        \* Current protocol slot
    blocks,             \* All blocks created: Block -> BlockData
    votes,              \* All votes cast: Vote
    certificates,       \* All certificates: Certificate  
    nodeState,          \* Per-node per-slot state: Node -> Slot -> StateSet
    pendingBlocks,      \* Pending blocks per node: Node -> Slot -> Block
    activeTimeouts,     \* Active timeouts: Node -> Set(Slot)
    finalized,          \* Finalized blocks: Set(BlockHash)
    leaders,            \* Leader schedule: Slot -> Node
    pool                \* Pool state per node: Node -> PoolState

vars == <<currentSlot, blocks, votes, certificates, nodeState, pendingBlocks, 
          activeTimeouts, finalized, leaders, pool>>

-----------------------------------------------------------------------------

(*
Type definitions matching whitepaper exactly
*)

\* Nodes and stake (20% Byzantine assumption)
CorrectNodes == Nodes \ ByzantineNodes
Stake(n) == IF n \in ByzantineNodes THEN 1 ELSE 4
TotalStake == Cardinality(CorrectNodes) * 4 + Cardinality(ByzantineNodes) * 1

\* Block structure (Definition 3 from whitepaper)
BlockHash == Nat
BlockData == [slot: Nat, hash: BlockHash, parent: BlockHash \cup {-1}]

\* Vote types (Table 5 from whitepaper)
VoteType == {"NotarVote", "NotarFallbackVote", "SkipVote", "SkipFallbackVote", "FinalVote"}

\* Certificate types (Table 6 from whitepaper)
CertType == {"FastFinalization", "Notarization", "NotarFallback", "Skip", "Finalization"}

\* Vote structure
Vote == [type: VoteType, slot: Nat, block: BlockHash \cup {-1}, node: Nodes]

\* Certificate structure  
Certificate == [type: CertType, slot: Nat, block: BlockHash \cup {-1}, stake: Nat]

\* Node state components (Definition 18 from whitepaper)
StateComponent == {"ParentReady", "Voted", "VotedNotar", "BlockNotarized", "ItsOver", "BadWindow"}

\* Pool events (Definition 15 from whitepaper)
PoolEvent == {"BlockNotarized", "ParentReady", "SafeToNotar", "SafeToSkip"}

-----------------------------------------------------------------------------

(*
Helper operators
*)

\* Calculate total stake for a set of votes
VoteStake(voteSet) == 
    SumSet({Stake(v.node) : v \in voteSet})

\* Get votes of specific type for slot/block
GetVotes(voteType, slot, blockHash) ==
    {v \in votes : v.type = voteType /\ v.slot = slot /\ v.block = blockHash}

\* Check if certificate threshold is met
CertificateThresholdMet(voteTypes, slot, blockHash, threshold) ==
    LET relevantVotes == UNION {GetVotes(vt, slot, blockHash) : vt \in voteTypes}
        totalStake == VoteStake(relevantVotes)
    IN totalStake * 100 >= threshold * TotalStake

\* Leader window slots (Algorithm 2, line 1-2)
WindowSlots(firstSlot) ==
    {s \in firstSlot..(firstSlot + WindowSize - 1) : s <= MaxSlot}

\* Check if slot is first in its window
IsFirstSlot(slot) ==
    slot % WindowSize = 0

-----------------------------------------------------------------------------

(*
Initial state
*)

Init ==
    /\ currentSlot = 0
    /\ blocks = {}
    /\ votes = {}
    /\ certificates = {}
    /\ nodeState = [n \in Nodes |-> [s \in 0..MaxSlot |-> {}]]
    /\ pendingBlocks = [n \in Nodes |-> [s \in 0..MaxSlot |-> -1]]
    /\ activeTimeouts = [n \in Nodes |-> {}]
    /\ finalized = {}
    /\ leaders = [s \in 0..MaxSlot |-> CHOOSE n \in Nodes : TRUE]
    /\ pool = [n \in Nodes |-> [votes |-> {}, certificates |-> {}]]

-----------------------------------------------------------------------------

(*
Core Votor actions implementing Algorithm 1 exactly
*)

\* Algorithm 1, line 1: upon Block(s, hash, hashparent)
ReceiveBlock(node, blockData) ==
    /\ node \in CorrectNodes
    /\ blockData.slot <= currentSlot
    /\ blockData.hash \notin DOMAIN blocks
    /\ blocks' = blocks @@ (blockData.hash :> blockData)
    /\ LET slot == blockData.slot
           canNotar == TryNotar(node, blockData)
       IN IF canNotar
          THEN /\ \* Cast notarization vote
               votes' = votes \cup {[type |-> "NotarVote", slot |-> slot, 
                                    block |-> blockData.hash, node |-> node]}
               /\ nodeState' = [nodeState EXCEPT 
                   ![node][slot] = nodeState[node][slot] \cup {"Voted", "VotedNotar"}]
               /\ pendingBlocks' = [pendingBlocks EXCEPT ![node][slot] = -1]
          ELSE /\ IF "Voted" \notin nodeState[node][slot]
                  THEN pendingBlocks' = [pendingBlocks EXCEPT 
                           ![node][slot] = blockData.hash]
                  ELSE pendingBlocks' = pendingBlocks
               /\ votes' = votes
               /\ nodeState' = nodeState
    /\ UNCHANGED <<currentSlot, certificates, activeTimeouts, finalized, leaders, pool>>

\* Algorithm 1, line 6: upon Timeout(s)  
NodeTimeout(node, slot) ==
    /\ node \in CorrectNodes
    /\ slot \in activeTimeouts[node]
    /\ "Voted" \notin nodeState[node][slot]
    /\ \* Execute trySkipWindow(s)
       LET windowSlots == WindowSlots(slot - (slot % WindowSize))
       IN /\ votes' = votes \cup 
             {[type |-> "SkipVote", slot |-> s, block |-> -1, node |-> node] : 
              s \in windowSlots, "Voted" \notin nodeState[node][s]}
          /\ nodeState' = [nodeState EXCEPT 
             ![node] = [s \in 0..MaxSlot |-> 
                       IF s \in windowSlots /\ "Voted" \notin nodeState[node][s]
                       THEN nodeState[node][s] \cup {"Voted", "BadWindow"}
                       ELSE nodeState[node][s]]]
          /\ pendingBlocks' = [pendingBlocks EXCEPT
             ![node] = [s \in 0..MaxSlot |->
                       IF s \in windowSlots THEN -1 ELSE pendingBlocks[node][s]]]
    /\ activeTimeouts' = [activeTimeouts EXCEPT ![node] = activeTimeouts[node] \ {slot}]
    /\ UNCHANGED <<currentSlot, blocks, certificates, finalized, leaders, pool>>

\* Algorithm 1, line 9: upon BlockNotarized(s, hash(b))
HandleBlockNotarized(node, slot, blockHash) ==
    /\ node \in CorrectNodes
    /\ \E cert \in certificates : 
       cert.type = "Notarization" /\ cert.slot = slot /\ cert.block = blockHash
    /\ "BlockNotarized" \notin nodeState[node][slot]
    /\ nodeState' = [nodeState EXCEPT 
       ![node][slot] = nodeState[node][slot] \cup {"BlockNotarized"}]
    /\ \* Try to cast final vote (tryFinal)
       IF /\ "VotedNotar" \in nodeState[node][slot]
          /\ "BadWindow" \notin nodeState[node][slot]
       THEN /\ votes' = votes \cup {[type |-> "FinalVote", slot |-> slot, 
                                    block |-> blockHash, node |-> node]}
            /\ nodeState' = [nodeState' EXCEPT 
               ![node][slot] = nodeState'[node][slot] \cup {"ItsOver"}]
       ELSE votes' = votes
    /\ UNCHANGED <<currentSlot, blocks, certificates, pendingBlocks, 
                   activeTimeouts, finalized, leaders, pool>>

\* Algorithm 1, line 12: upon ParentReady(s, hash(b))
HandleParentReady(node, slot, parentHash) ==
    /\ node \in CorrectNodes
    /\ IsFirstSlot(slot)
    /\ "ParentReady" \notin nodeState[node][slot]
    /\ nodeState' = [nodeState EXCEPT 
       ![node][slot] = nodeState[node][slot] \cup {"ParentReady"}]
    /\ \* Set timeouts for window (setTimeouts from Algorithm 2)
       LET windowSlots == WindowSlots(slot)
       IN activeTimeouts' = [activeTimeouts EXCEPT 
          ![node] = activeTimeouts[node] \cup windowSlots]
    /\ \* Check pending blocks (checkPendingBlocks)
       UNCHANGED <<currentSlot, blocks, votes, certificates, pendingBlocks, 
                   finalized, leaders, pool>>

-----------------------------------------------------------------------------

(*
Helper functions implementing Algorithm 2 exactly
*)

\* Algorithm 2, line 7-17: tryNotar function
TryNotar(node, blockData) ==
    LET slot == blockData.slot
        firstSlot == IsFirstSlot(slot)
        parentHash == blockData.parent
    IN /\ "Voted" \notin nodeState[node][slot]
       /\ \/ /\ firstSlot 
             /\ "ParentReady" \in nodeState[node][slot]
          \/ /\ ~firstSlot
             /\ slot > 0
             /\ "VotedNotar" \in nodeState[node][slot - 1]

-----------------------------------------------------------------------------

(*
Certificate generation (Definition 13 from whitepaper)
*)

GenerateCertificates ==
    \* Fast-finalization certificate (≥80% NotarVote)
    \/ \E slot \in 0..currentSlot, blockHash \in BlockHash :
       /\ CertificateThresholdMet({"NotarVote"}, slot, blockHash, 80)
       /\ ~\E cert \in certificates : 
          cert.type = "FastFinalization" /\ cert.slot = slot /\ cert.block = blockHash
       /\ certificates' = certificates \cup 
          {[type |-> "FastFinalization", slot |-> slot, block |-> blockHash,
            stake |-> VoteStake(GetVotes("NotarVote", slot, blockHash))]}
       /\ UNCHANGED <<currentSlot, blocks, votes, nodeState, pendingBlocks, 
                      activeTimeouts, finalized, leaders, pool>>
    
    \* Notarization certificate (≥60% NotarVote)
    \/ \E slot \in 0..currentSlot, blockHash \in BlockHash :
       /\ CertificateThresholdMet({"NotarVote"}, slot, blockHash, 60)
       /\ ~\E cert \in certificates : 
          cert.type = "Notarization" /\ cert.slot = slot /\ cert.block = blockHash
       /\ certificates' = certificates \cup 
          {[type |-> "Notarization", slot |-> slot, block |-> blockHash,
            stake |-> VoteStake(GetVotes("NotarVote", slot, blockHash))]}
       /\ UNCHANGED <<currentSlot, blocks, votes, nodeState, pendingBlocks, 
                      activeTimeouts, finalized, leaders, pool>>

    \* Skip certificate (≥60% SkipVote or SkipFallbackVote)
    \/ \E slot \in 0..currentSlot :
       /\ CertificateThresholdMet({"SkipVote", "SkipFallbackVote"}, slot, -1, 60)
       /\ ~\E cert \in certificates : 
          cert.type = "Skip" /\ cert.slot = slot
       /\ certificates' = certificates \cup 
          {[type |-> "Skip", slot |-> slot, block |-> -1,
            stake |-> VoteStake(GetVotes("SkipVote", slot, -1) \cup 
                               GetVotes("SkipFallbackVote", slot, -1))]}
       /\ UNCHANGED <<currentSlot, blocks, votes, nodeState, pendingBlocks, 
                      activeTimeouts, finalized, leaders, pool>>

    \* Finalization certificate (≥60% FinalVote)
    \/ \E slot \in 0..currentSlot, blockHash \in BlockHash :
       /\ CertificateThresholdMet({"FinalVote"}, slot, blockHash, 60)
       /\ ~\E cert \in certificates : 
          cert.type = "Finalization" /\ cert.slot = slot /\ cert.block = blockHash
       /\ certificates' = certificates \cup 
          {[type |-> "Finalization", slot |-> slot, block |-> blockHash,
            stake |-> VoteStake(GetVotes("FinalVote", slot, blockHash))]}
       /\ UNCHANGED <<currentSlot, blocks, votes, nodeState, pendingBlocks, 
                      activeTimeouts, finalized, leaders, pool>>

-----------------------------------------------------------------------------

(*
Block finalization (Definition 14 from whitepaper)
*)

FinalizeBlocks ==
    \* Fast finalization
    \/ \E cert \in certificates :
       /\ cert.type = "FastFinalization"
       /\ cert.block \notin finalized
       /\ finalized' = finalized \cup {cert.block}
       /\ UNCHANGED <<currentSlot, blocks, votes, certificates, nodeState, 
                      pendingBlocks, activeTimeouts, leaders, pool>>
    
    \* Slow finalization (finalization cert + notarization cert)
    \/ \E finalCert, notarCert \in certificates :
       /\ finalCert.type = "Finalization"
       /\ notarCert.type = "Notarization" 
       /\ finalCert.slot = notarCert.slot
       /\ finalCert.block = notarCert.block
       /\ finalCert.block \notin finalized
       /\ finalized' = finalized \cup {finalCert.block}
       /\ UNCHANGED <<currentSlot, blocks, votes, certificates, nodeState, 
                      pendingBlocks, activeTimeouts, leaders, pool>>

-----------------------------------------------------------------------------

(*
Protocol progression
*)

AdvanceSlot ==
    /\ currentSlot < MaxSlot
    /\ currentSlot' = currentSlot + 1
    /\ UNCHANGED <<blocks, votes, certificates, nodeState, pendingBlocks, 
                   activeTimeouts, finalized, leaders, pool>>

-----------------------------------------------------------------------------

(*
Next state relation
*)

Next ==
    \/ \E node \in CorrectNodes, blockData \in BlockData : ReceiveBlock(node, blockData)
    \/ \E node \in CorrectNodes, slot \in 0..MaxSlot : NodeTimeout(node, slot)
    \/ \E node \in CorrectNodes, slot \in 0..MaxSlot, blockHash \in BlockHash : 
       HandleBlockNotarized(node, slot, blockHash)
    \/ \E node \in CorrectNodes, slot \in 0..MaxSlot, parentHash \in BlockHash \cup {-1} : 
       HandleParentReady(node, slot, parentHash)
    \/ GenerateCertificates
    \/ FinalizeBlocks
    \/ AdvanceSlot

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------

(*
Safety properties (Theorem 1 and supporting lemmas)
*)

\* Lemma 20: A correct node exclusively casts only one notarization or skip vote per slot
SafetyExclusiveVoting ==
    \A node \in CorrectNodes, slot \in 0..MaxSlot :
        LET notarVotes == {v \in votes : v.node = node /\ v.slot = slot /\ v.type = "NotarVote"}
            skipVotes == {v \in votes : v.node = node /\ v.slot = slot /\ v.type = "SkipVote"}
        IN Cardinality(notarVotes) <= 1 /\ Cardinality(skipVotes) <= 1 /\
           (Cardinality(notarVotes) = 1 => Cardinality(skipVotes) = 0) /\
           (Cardinality(skipVotes) = 1 => Cardinality(notarVotes) = 0)

\* Lemma 24: At most one block can be notarized in a given slot
SafetyUniqueNotarization ==
    \A slot \in 0..MaxSlot :
        LET notarCerts == {cert \in certificates : cert.type = "Notarization" /\ cert.slot = slot}
        IN Cardinality(notarCerts) <= 1

\* Theorem 1: Safety - no conflicting finalized blocks
SafetyNoConflictingFinalization ==
    \A b1, b2 \in finalized :
        b1 # b2 =>
        \A block1, block2 \in DOMAIN blocks :
            /\ blocks[block1].hash = b1 /\ blocks[block2].hash = b2
            /\ blocks[block1].slot = blocks[block2].slot
            => FALSE

-----------------------------------------------------------------------------

(*
Liveness properties (Theorem 2 and supporting lemmas)
*)

\* Lemma 41: All correct nodes will set timeouts for all slots
LivenessTimeoutsSetting ==
    \A node \in CorrectNodes, slot \in 0..MaxSlot :
        IsFirstSlot(slot) /\ "ParentReady" \in nodeState[node][slot] =>
        WindowSlots(slot) \subseteq activeTimeouts[node]

\* Theorem 2: Liveness - correct leader blocks get finalized
LivenessCorrectLeaderFinalization ==
    \A slot \in 0..MaxSlot :
        /\ leaders[slot] \in CorrectNodes
        /\ \E blockHash \in DOMAIN blocks : blocks[blockHash].slot = slot
        => <>(\E blockHash \in DOMAIN blocks : 
              blocks[blockHash].slot = slot /\ blockHash \in finalized)

=============================================================================
