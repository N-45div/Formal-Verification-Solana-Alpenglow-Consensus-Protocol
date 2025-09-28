graph TD
    subgraph "Leader Node"
        L[Block Data<br/>1MB]
        L --> EC[Erasure Coding<br/>κ = 5/3 expansion]
        EC --> S1[Shred 1]
        EC --> S2[Shred 2]
        EC --> S3[Shred 3]
        EC --> S4[Shred 4]
        EC --> S5[Shred 5]
    end
    
    subgraph "Relay Network"
        R1[Relay 1<br/>High Stake]
        R2[Relay 2<br/>Medium Stake]
        R3[Relay 3<br/>Low Stake]
        R4[Relay 4<br/>High Stake]
        R5[Relay 5<br/>Medium Stake]
    end
    
    S1 --> R1
    S2 --> R2
    S3 --> R3
    S4 --> R4
    S5 --> R5
    
    subgraph "Validator Nodes"
        V1[Validator 1]
        V2[Validator 2]
        V3[Validator 3]
        V4[Validator 4]
    end
    
    R1 --> V1
    R1 --> V2
    R2 --> V2
    R2 --> V3
    R3 --> V3
    R3 --> V4
    R4 --> V4
    R4 --> V1
    R5 --> V1
    R5 --> V2
    
    subgraph "Reconstruction"
        V1 --> REC1[Reconstruct<br/>Need ≥3 shreds]
        V2 --> REC2[Reconstruct<br/>Need ≥3 shreds]
        V3 --> REC3[Reconstruct<br/>Need ≥3 shreds]
        V4 --> REC4[Reconstruct<br/>Need ≥3 shreds]
    end
    
    style L fill:#4A90E2
    style EC fill:#7B68EE
    style REC1 fill:#90EE90
    style REC2 fill:#90EE90
    style REC3 fill:#90EE90
    style REC4 fill:#90EE90