#![allow(mixed_script_confusables)]
#![verifier::loop_isolation(false)]
#![allow(non_camel_case_types)]
use vstd::{prelude::*};
use std::vec::Vec;
mod lock;

verus! {

const NUM_REPLICAS : u64 = 37;
type StateType = u64;
type Error = u64;
const ENone : u64 = 0u64;
const EEpochStale : u64 = 1u64;
const ENotLeader : u64 = 2u64;

////////////////////////////////////////////////////////////////////////////////
// Acceptor

struct AcceptorState {
    epoch : u64,
    accepted_epoch : u64,
    next_index : u64,
    state : StateType,
}

type AcceptorResource = u64;

pub struct Acceptor {
    mu: lock::Lock<(AcceptorState, Tracked<AcceptorResource>)>,
}

#[derive(Clone, Copy)]
struct AcceptorApplyArgs {
    epoch:u64,
    next_index:u64,
    state:StateType,
}

type AcceptorApplyReply = u64;

impl Acceptor {
    spec fn inv(self) -> bool {
        forall |s| #[trigger] self.mu.get_pred()(s) <==> true // server_inv(s.0, s.1@)
    }

    fn apply(&self, args:AcceptorApplyArgs) -> (ret:AcceptorApplyReply)
        requires self.inv()
    {
        let (mut s, mut res) = self.mu.lock();
        let mut reply;

        if s.epoch <= args.epoch {
            if s.accepted_epoch == args.epoch {
                if s.next_index < args.next_index {
                    s.next_index = args.next_index;
                    s.state = args.state;
                    reply = ENone;
                } else { // args.next_index < s.next_index
                    reply = ENone;
                }
            } else { // s.accepted_epoch < args.epoch, because s.accepted_epoch <= s.epoch <= args.epoch
                s.accepted_epoch = args.epoch;
                s.epoch = args.epoch;
                s.state = args.state;
                s.next_index = args.next_index;
                reply = ENone;
            }
        } else {
            reply = EEpochStale;
        }

        self.mu.unlock((s, res));
        return 0;
    }

}

////////////////////////////////////////////////////////////////////////////////
// Proposer

struct ProposerState {
    epoch : u64,
    next_index : u64,
    state : StateType,

    is_leader : bool,
    clerks : Vec<&'static Acceptor>,
}

type ProposerResource = u64;

pub struct Proposer {
    mu: lock::Lock<(ProposerState, Tracked<ProposerResource>)>,
}

spec fn proposer_inv(s:ProposerState, res:ProposerResource) -> bool {
    &&& s.clerks@.len() == NUM_REPLICAS
    &&& forall |j| 0 <= j < s.clerks.len() ==> #[trigger] s.clerks[j].inv()
}

impl Proposer {
    spec fn inv(self) -> bool {
        forall |s| #[trigger] self.mu.get_pred()(s) <==> proposer_inv(s.0, s.1@)
    }

    // TODO: take as input a "step" function
    fn try_apply(&self) -> Error
        requires self.inv()
    {
        let (mut s, mut res) = self.mu.lock();
        if !s.is_leader {
            self.mu.unlock((s, res));
            return ENotLeader;
        }

        let mut num_successes = 0u64;
        let args = AcceptorApplyArgs{
            epoch: s.epoch,
            next_index: s.next_index,
            state: s.state,
        };

        let mut i : usize = 0;

        while i < NUM_REPLICAS as usize
            invariant
              0 <= i <= NUM_REPLICAS,
              0 <= num_successes <= i,
        {
            let reply = s.clerks[i].apply(args);
            if reply == ENone {
                num_successes += 1;
            }
            i += 1;
        }

        if 2*num_successes > NUM_REPLICAS {
            return ENone
        }
        return EEpochStale
    }
}

////////////////////////////////////////////////////////////////////////////////
// General definitions

type True = ();
enum Or<A,B> {Left(A), Right(B)}

trait iProp {
    spec fn pure_prop(Self) -> bool;
}

#[verifier(external_body)]
#[verifier::reject_recursive_types(P)]
#[verifier::reject_recursive_types(Q)]
struct ⟦wand⟧<⟦P⟧,⟦Q⟧> {
    _phantom1 : std::marker::PhantomData<⟦P⟧>,
    _phantom2 : std::marker::PhantomData<⟦Q⟧>,
}
spec fn ⟨wand⟩<⟦P⟧,⟦Q⟧>(
    ⟨P⟩:spec_fn(⟦P⟧) -> bool,
    ⟨Q⟩:spec_fn(⟦Q⟧) -> bool
) -> bool;
impl<P,Q> ⟦wand⟧<P,Q> {
    #[verifier(external_body)]
    proof fn instantiate(i:P) -> (out:Q)
        requires
          exists |⟨P⟩, ⟨Q⟩| { &&&
            ⟨wand⟩(⟨P⟩,⟨Q⟩) &&&
            ⟨P⟩(i)
          }

        ensures
          forall |⟨P⟩, ⟨Q⟩| ⟨wand⟩(⟨P⟩,⟨Q⟩) ==> ⟨Q⟩(out)

        opens_invariants none
    {
        unimplemented!();
    }
}

////////////////////////////////////////////////////////////////////////////////
// General resources

type gname = nat;

#[verifier(external_body)]
struct ⟦tok_points_to⟧ {
}
spec fn ⟨tok_points_to⟩(gamma:gname, k:u64) -> spec_fn(⟦tok_points_to⟧) -> bool;


#[verifier(external_body)]
#[verifier::reject_recursive_types(K)]
#[verifier::reject_recursive_types(T)]
struct ⟦mlist_ptsto⟧<K,T> {
    _phantom1 : std::marker::PhantomData<K>,
    _phantom2 : std::marker::PhantomData<T>,
}
spec fn ⟨mlist_ptsto⟩<K,T>(γ:gname, key:u64, l:Seq<T>) -> spec_fn(⟦mlist_ptsto⟧<K,T>) -> bool;


#[verifier(external_body)]
#[verifier::reject_recursive_types(K)]
#[verifier::reject_recursive_types(T)]
struct ⟦mlist_ptsto_lb⟧<K,T> {
    _phantom1 : std::marker::PhantomData<K>,
    _phantom2 : std::marker::PhantomData<T>,
}
spec fn ⟨mlist_ptsto_lb⟩<K,T>(γ:gname, key:u64, l:Seq<T>) -> spec_fn(⟦mlist_ptsto⟧<K,T>) -> bool;

                 
#[verifier(external_body)]
#[verifier::reject_recursive_types(K)]
#[verifier::reject_recursive_types(T)]
struct ⟦mlist_ptsto_ro⟧<K,T> {
    _phantom1 : std::marker::PhantomData<K>,
    _phantom2 : std::marker::PhantomData<T>,
}
spec fn ⟨mlist_ptsto_ro⟩<K,T>(γ:gname, key:u64, l:Seq<T>) -> spec_fn(⟦mlist_ptsto⟧<K,T>) -> bool;

////////////////////////////////////////////////////////////////////////////////
// Paxos separation logic theory

struct mp_system_names {
    proposal_gn : gname,
    state_gn : gname,
}

struct mp_server_names {
    accepted_gn : gname,
    vote_gn : gname,
}


#[verifier::reject_recursive_types(K)]
struct ⟦big_sepS⟧<K,R> {
    contents: Map<K,R>
}
spec fn ⟨big_sepS⟩<K,R>(s:Set<K>, f:spec_fn(K) -> spec_fn(R) -> bool)
                        -> spec_fn(⟦big_sepS⟧<K,R>) -> bool {
    |res:⟦big_sepS⟧<_,_>| {
        &&& res.contents.dom() == s
        &&& forall |k| #[trigger] s.contains(k) ==> f(k)(res.contents[k])
    }
}


type EntryType = StateType;
type ⟦is_proposal_lb⟧ = ⟦mlist_ptsto_lb⟧<u64, EntryType>;
type ⟦is_proposal_facts⟧ = ⟦mlist_ptsto_lb⟧<u64, EntryType>; // FIXME: wrong type
type ⟦is_accepted_lb⟧ = ⟦mlist_ptsto_lb⟧<u64, EntryType>;
type ⟦own_accepted⟧ = ⟦mlist_ptsto⟧<u64, EntryType>;
type ⟦is_accepted_ro⟧ = ⟦mlist_ptsto_ro⟧<u64, EntryType>;


type ⟦own_vote_tok⟧ = ⟦tok_points_to⟧;
spec fn ⟨own_vote_tok⟩(γsrv:mp_server_names, epoch:u64) -> spec_fn(⟦own_vote_tok⟧) -> bool {
    |res| {
        ⟨tok_points_to⟩(γsrv.vote_gn, epoch)(res)
    }
}

type ⟦is_accepted_upper_bound⟧ = (
    ⟦is_accepted_ro⟧,
    // □(dyn_Atomic_update)
);


struct ⟦own_replica_ghost⟧ {
    Hprop_lb : ⟦is_proposal_lb⟧,
    Hprop_facts : ⟦is_proposal_facts⟧,
    Hacc_lb : ⟦is_accepted_lb⟧,
    HepochIneq : (), // pure
    Hacc : ⟦own_accepted⟧,
    Hacc_ub : Or<True, ⟦is_accepted_upper_bound⟧>,
    Hunused : ⟦big_sepS⟧<u64, ()>,
    Hvotes : ⟦big_sepS⟧<u64, ⟦own_vote_tok⟧>,
}
    
fn main() {}
}
