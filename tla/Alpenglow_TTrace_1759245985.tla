---- MODULE Alpenglow_TTrace_1759245985 ----
EXTENDS Sequences, TLCExt, Alpenglow_TEConstants, Toolbox, Naturals, TLC, Alpenglow

_expression ==
    LET Alpenglow_TEExpression == INSTANCE Alpenglow_TEExpression
    IN Alpenglow_TEExpression!expression
----

_trace ==
    LET Alpenglow_TETrace == INSTANCE Alpenglow_TETrace
    IN Alpenglow_TETrace!trace
----

_prop ==
    ~<>[](
        finalized = ({})
        /\
        slots = (2)
        /\
        certificates = ({})
        /\
        blocks = ({[slot |-> 0, hash |-> 1, parent |-> 1]})
        /\
        timeouts = ((n1 :> {} @@ n2 :> {} @@ n3 :> {}))
        /\
        leaders = ((0 :> n1 @@ 1 :> n1 @@ 2 :> n1))
        /\
        relayGraph = ((n1 :> {} @@ n2 :> {} @@ n3 :> {}))
        /\
        votes = ({[slot |-> 0, type |-> "NotarVote", block |-> 1, node |-> n3]})
        /\
        shreds = ({})
        /\
        nodeStates = ((n1 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n2 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n3 :> (0 :> {"Voted", "VotedNotar"} @@ 1 :> {} @@ 2 :> {})))
    )
----

_init ==
    /\ leaders = _TETrace[1].leaders
    /\ timeouts = _TETrace[1].timeouts
    /\ slots = _TETrace[1].slots
    /\ relayGraph = _TETrace[1].relayGraph
    /\ blocks = _TETrace[1].blocks
    /\ votes = _TETrace[1].votes
    /\ shreds = _TETrace[1].shreds
    /\ certificates = _TETrace[1].certificates
    /\ nodeStates = _TETrace[1].nodeStates
    /\ finalized = _TETrace[1].finalized
----

_next ==
    /\ \E i,j \in DOMAIN _TETrace:
        /\ \/ /\ j = i + 1
              /\ i = TLCGet("level")
        /\ leaders  = _TETrace[i].leaders
        /\ leaders' = _TETrace[j].leaders
        /\ timeouts  = _TETrace[i].timeouts
        /\ timeouts' = _TETrace[j].timeouts
        /\ slots  = _TETrace[i].slots
        /\ slots' = _TETrace[j].slots
        /\ relayGraph  = _TETrace[i].relayGraph
        /\ relayGraph' = _TETrace[j].relayGraph
        /\ blocks  = _TETrace[i].blocks
        /\ blocks' = _TETrace[j].blocks
        /\ votes  = _TETrace[i].votes
        /\ votes' = _TETrace[j].votes
        /\ shreds  = _TETrace[i].shreds
        /\ shreds' = _TETrace[j].shreds
        /\ certificates  = _TETrace[i].certificates
        /\ certificates' = _TETrace[j].certificates
        /\ nodeStates  = _TETrace[i].nodeStates
        /\ nodeStates' = _TETrace[j].nodeStates
        /\ finalized  = _TETrace[i].finalized
        /\ finalized' = _TETrace[j].finalized

\* Uncomment the ASSUME below to write the states of the error trace
\* to the given file in Json format. Note that you can pass any tuple
\* to `JsonSerialize`. For example, a sub-sequence of _TETrace.
    \* ASSUME
    \*     LET J == INSTANCE Json
    \*         IN J!JsonSerialize("Alpenglow_TTrace_1759245985.json", _TETrace)

=============================================================================

 Note that you can extract this module `Alpenglow_TEExpression`
  to a dedicated file to reuse `expression` (the module in the 
  dedicated `Alpenglow_TEExpression.tla` file takes precedence 
  over the module `Alpenglow_TEExpression` below).

---- MODULE Alpenglow_TEExpression ----
EXTENDS Sequences, TLCExt, Alpenglow_TEConstants, Toolbox, Naturals, TLC, Alpenglow

expression == 
    [
        \* To hide variables of the `Alpenglow` spec from the error trace,
        \* remove the variables below.  The trace will be written in the order
        \* of the fields of this record.
        leaders |-> leaders
        ,timeouts |-> timeouts
        ,slots |-> slots
        ,relayGraph |-> relayGraph
        ,blocks |-> blocks
        ,votes |-> votes
        ,shreds |-> shreds
        ,certificates |-> certificates
        ,nodeStates |-> nodeStates
        ,finalized |-> finalized
        
        \* Put additional constant-, state-, and action-level expressions here:
        \* ,_stateNumber |-> _TEPosition
        \* ,_leadersUnchanged |-> leaders = leaders'
        
        \* Format the `leaders` variable as Json value.
        \* ,_leadersJson |->
        \*     LET J == INSTANCE Json
        \*     IN J!ToJson(leaders)
        
        \* Lastly, you may build expressions over arbitrary sets of states by
        \* leveraging the _TETrace operator.  For example, this is how to
        \* count the number of times a spec variable changed up to the current
        \* state in the trace.
        \* ,_leadersModCount |->
        \*     LET F[s \in DOMAIN _TETrace] ==
        \*         IF s = 1 THEN 0
        \*         ELSE IF _TETrace[s].leaders # _TETrace[s-1].leaders
        \*             THEN 1 + F[s-1] ELSE F[s-1]
        \*     IN F[_TEPosition - 1]
    ]

=============================================================================



Parsing and semantic processing can take forever if the trace below is long.
 In this case, it is advised to uncomment the module below to deserialize the
 trace from a generated binary file.

\*
\*---- MODULE Alpenglow_TETrace ----
\*EXTENDS IOUtils, Alpenglow_TEConstants, TLC, Alpenglow
\*
\*trace == IODeserialize("Alpenglow_TTrace_1759245985.bin", TRUE)
\*
\*=============================================================================
\*

---- MODULE Alpenglow_TETrace ----
EXTENDS Alpenglow_TEConstants, TLC, Alpenglow

trace == 
    <<
    ([finalized |-> {},slots |-> 0,certificates |-> {},blocks |-> {},timeouts |-> (n1 :> {} @@ n2 :> {} @@ n3 :> {}),leaders |-> (0 :> n1 @@ 1 :> n1 @@ 2 :> n1),relayGraph |-> (n1 :> {} @@ n2 :> {} @@ n3 :> {}),votes |-> {},shreds |-> {},nodeStates |-> (n1 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n2 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n3 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}))]),
    ([finalized |-> {},slots |-> 0,certificates |-> {},blocks |-> {[slot |-> 0, hash |-> 1, parent |-> 1]},timeouts |-> (n1 :> {} @@ n2 :> {} @@ n3 :> {}),leaders |-> (0 :> n1 @@ 1 :> n1 @@ 2 :> n1),relayGraph |-> (n1 :> {} @@ n2 :> {} @@ n3 :> {}),votes |-> {[slot |-> 0, type |-> "NotarVote", block |-> 1, node |-> n3]},shreds |-> {},nodeStates |-> (n1 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n2 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n3 :> (0 :> {"Voted", "VotedNotar"} @@ 1 :> {} @@ 2 :> {}))]),
    ([finalized |-> {},slots |-> 1,certificates |-> {},blocks |-> {[slot |-> 0, hash |-> 1, parent |-> 1]},timeouts |-> (n1 :> {} @@ n2 :> {} @@ n3 :> {}),leaders |-> (0 :> n1 @@ 1 :> n1 @@ 2 :> n1),relayGraph |-> (n1 :> {} @@ n2 :> {} @@ n3 :> {}),votes |-> {[slot |-> 0, type |-> "NotarVote", block |-> 1, node |-> n3]},shreds |-> {},nodeStates |-> (n1 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n2 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n3 :> (0 :> {"Voted", "VotedNotar"} @@ 1 :> {} @@ 2 :> {}))]),
    ([finalized |-> {},slots |-> 2,certificates |-> {},blocks |-> {[slot |-> 0, hash |-> 1, parent |-> 1]},timeouts |-> (n1 :> {} @@ n2 :> {} @@ n3 :> {}),leaders |-> (0 :> n1 @@ 1 :> n1 @@ 2 :> n1),relayGraph |-> (n1 :> {} @@ n2 :> {} @@ n3 :> {}),votes |-> {[slot |-> 0, type |-> "NotarVote", block |-> 1, node |-> n3]},shreds |-> {},nodeStates |-> (n1 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n2 :> (0 :> {} @@ 1 :> {} @@ 2 :> {}) @@ n3 :> (0 :> {"Voted", "VotedNotar"} @@ 1 :> {} @@ 2 :> {}))])
    >>
----


=============================================================================

---- MODULE Alpenglow_TEConstants ----
EXTENDS Alpenglow

CONSTANTS n1, n2, n3

=============================================================================

---- CONFIG Alpenglow_TTrace_1759245985 ----
CONSTANTS
    Nodes = { n1 , n2 , n3 }
    MaxSlot = 2
    WindowSize = 1
    ByzantineNodes = { }
    Delta = 1
    DeltaTimeout = 2
    n3 = n3
    n1 = n1
    n2 = n2

PROPERTY
    _prop

CHECK_DEADLOCK
    \* CHECK_DEADLOCK off because of PROPERTY or INVARIANT above.
    FALSE

INIT
    _init

NEXT
    _next

CONSTANT
    _TETrace <- _trace

ALIAS
    _expression
=============================================================================
\* Generated on Tue Sep 30 15:26:29 UTC 2025