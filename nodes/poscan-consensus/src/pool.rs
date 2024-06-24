use sp_std::collections::{
    vec_deque::VecDeque,
    btree_map::BTreeMap,
    btree_set::BTreeSet,
};
use ecies_ed25519;
use sp_std::sync::Arc;
use sp_core::{H256, U256};
use parking_lot::{Mutex, Condvar};
use poscan_grid2d::{hash_meets_difficulty, DoubleHash, ComputeV2};
use poscan_algo::get_obj_hashes;
use runtime::AccountId;

use sp_consensus_poscan::CheckMemberError;

const MAX_QUEUE_LEN: usize = 10;
pub(crate) const LOG_TARGET: &'static str = "mining-pool";

pub struct ShareProposal {
    pub(crate) member_id: AccountId,
    pub(crate) pre_obj: Vec<u8>,
    pub(crate) algo_type: [u8; 16],
    pub(crate) hash: H256,
    pub(crate) pre_hash: H256,
    pub(crate) share_dfclty: U256,
    pub(crate) parent_hash: H256,
}

#[derive(Clone, PartialEq)]
pub struct MiningMeta {
    pub(crate) pre_hash:    H256,
    pub(crate) parent_hash: H256,
    pub(crate) difficulty:  U256,
}

pub enum PoolError {
    NotAccepted,
    CheckMemberError(CheckMemberError),
    InvalidMemberError,
    SignatureError,
    SignatureMemberError,
}

pub type PoolResult<T> = std::result::Result<T, PoolError>;

// #[derive(Clone)]
pub struct MiningPool {
    pub curr_meta:   Option<MiningMeta>,
    pub prev_meta:   Option<MiningMeta>,
    pub queue:       Arc<Mutex<VecDeque<ShareProposal>>>,
    pub cvar:        Arc<Condvar>,
    pub hist_hashes: BTreeMap<AccountId, BTreeSet<H256>>,
    pub (crate) secret:          ecies_ed25519::SecretKey,
    public:          ecies_ed25519::PublicKey,
}

impl Clone for MiningPool {
    fn clone(&self) -> Self {
        let secret = ecies_ed25519::SecretKey::from_bytes(&self.secret.to_bytes()).unwrap();
        Self {
            curr_meta: self.curr_meta.clone(),
            prev_meta: self.prev_meta.clone(),
            queue: self.queue.clone(),
            cvar: self.cvar.clone(),
            hist_hashes: self.hist_hashes.clone(),
            secret,
            public: self.public.clone(),
        }
    }
}

impl MiningPool {
    pub(crate) fn new() -> Self {
        let mut csprng = rand::thread_rng();
        let (secret, public) = ecies_ed25519::generate_keypair(&mut csprng);
        Self {
            curr_meta: None,
            prev_meta: None,
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_QUEUE_LEN))),
            cvar: Arc::new(Condvar::new()),
            hist_hashes: BTreeMap::new(),
            secret,
            public,
        }
    }

    pub(crate) fn update_metadata(&mut self, pre_hash: H256, parent_hash: H256, difficulty: U256) {
        let meta = MiningMeta{ pre_hash, parent_hash, difficulty };
        if self.curr_meta != Some(meta.clone()) {
            self.prev_meta = self.curr_meta.clone();
            self.curr_meta = Some(meta);
            self.hist_hashes = BTreeMap::new();
        }
    }

    pub(crate) fn take(&self) -> Option<ShareProposal> {
        let mut que = self.queue.lock();
        let sp = que.pop_front();
        self.cvar.notify_one();
        sp
    }

    pub(crate) fn try_push(&mut self, sp: ShareProposal, patch_rot: bool) -> PoolResult<()> {
        let member_set = self.hist_hashes.get_mut(&sp.member_id);
        if let Some(member_set) = member_set {
            if member_set.contains(&sp.hash) {
                log::info!(">>> pool verify: duplicated");
                return Err(PoolError::NotAccepted);
            }
            member_set.insert(sp.hash);
        }
        else {
            let bs = BTreeSet::from([sp.hash]);
            self.hist_hashes.insert(sp.member_id.clone(), bs);
        }

        let should_submit = self.pre_verify(
            &sp.algo_type,
            sp.hash,
            sp.pre_obj.as_slice(),
            sp.pre_hash,
            sp.parent_hash,
            sp.share_dfclty,
            patch_rot,
        )?;

        if should_submit {
            let mut que = self.queue.lock();

            while que.len() == MAX_QUEUE_LEN {
                self.cvar.wait(&mut que);
            }
            que.push_back(sp);
        }
        Ok(())
    }

    fn pre_verify(
        &self,
        alg_id: &[u8;16],
        hash: H256,
        obj: &[u8],
        pre_hash: H256,
        parent_hash:
        H256,
        dfclty: U256,
        patch_rot: bool,
    ) -> PoolResult<bool> {
        // type Signature = sp_core::sr25519::Signature;
        // type PublicKey = sp_core::sr25519::Public;

        // use sp_std::convert::TryFrom;
        // let sig = Signature::try_from(proof.as_slice()).unwrap();

        let dh = DoubleHash {pre_hash, obj_hash: hash}.calc_hash();

        // log::debug!(target: LOG_TARGET, "meta pre_hash {}", &self.curr_meta.as_ref().unwrap_or_default().pre_hash);
        // log::debug!(target: LOG_TARGET, "pool pre_hash {}", &pre_hash);
        // log::debug!(target: LOG_TARGET, "meta pre_hash {}", &self.curr_meta.as_ref().unwrap_or_default().parent_hash);
        // log::debug!(target: LOG_TARGET, "pool pre_hash {}", &parent_hash);

        let check =
        if let Some(curr_meta) = &self.curr_meta {
            curr_meta.pre_hash == pre_hash && curr_meta.parent_hash == parent_hash
            ||
            if let Some(prev_meta) = &self.prev_meta {
                prev_meta.pre_hash == pre_hash && prev_meta.parent_hash == parent_hash
            }
            else {
                log::debug!(">>> pool verify: provided pre_hash and parent_hash are not in metadata");
                false
            }
        }
        else {
            false
        };

        if !check {
            // TODO: different logs for errors
            log::info!(">>> pool verify: failed");
            return Err(PoolError::NotAccepted);
        }

        let hashes = get_obj_hashes(&alg_id, &Vec::from(obj), &pre_hash, patch_rot);

        if hashes.len() > 0 && hashes[0] != hash {
            log::info!(">>> pool verify: provided hashes are invalid");
            return Err(PoolError::NotAccepted);
        }

        // TODO:
        let comp = ComputeV2 {difficulty: self.curr_meta.as_ref().unwrap().difficulty, pre_hash, poscan_hash: dh, orig_hash: H256::default(), hist_hash: H256::default()};

        if hash_meets_difficulty(&comp.get_work(), self.curr_meta.as_ref().unwrap().difficulty) {
            Ok(true)
        }
        else {
            // TODO:
            let comp = ComputeV2 {difficulty: dfclty, pre_hash,  poscan_hash: dh, orig_hash: H256::default(), hist_hash: H256::default() };

            if hash_meets_difficulty(&comp.get_work(), dfclty) {
                Ok(false)
            }
            else {
                Err(PoolError::NotAccepted)
            }
        }
    }

    pub(crate) fn public(&self) -> U256 {
        let bytes = self.public.as_bytes();
        U256::from(bytes)
    }

}

// pub(crate) run_pool(mp: MiningPool) -> impl Future<Output = ()>{
//     loop {
//         let mut lock = mp.queue.lock();
//         if lock.len() >= MAX_QUEUE_LEN {
//             return Ok(RES_QUEUE_FULL);
//         }
//         if obj.len() > MAX_MINING_OBJ_LEN {
//             return Ok(RES_OBJ_MAX_LEN);
//         }
//         (*lock).push_back(MiningProposal { id: 1, pre_obj: obj.as_bytes().to_vec() });
//
//         Ok(RES_OK)
//     }
// }