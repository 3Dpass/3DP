# TC Candidates Pallet

## Overview

The **TC Candidates** pallet (`pallet_tc_candidates`) restricts Technical Committee (TC) membership to accounts that have been the **proposer** of an **enacted** runtime upgrade referendum (`System::set_code`). The list of eligible accounts (`EnactedSetCodeProposers`) is the TC candidate list itself: only this list is checked when adding TC members (via a runtime call filter). Candidates join either by calling **`submit_candidacy`** with a **state proof** that at a past block they were the proposer of a referendum that is now Approved and whose proposal was `set_code`, or by **Root** via **`add_candidate_force`** (bypasses the proof; for bootstrapping or recovery).

No hook in `pallet_referenda` (or elsewhere) is required. Proof generation is done via RPC; the runtime verifies the proof and adds the signer to `EnactedSetCodeProposers`.

---

## Design

### Storage

| Name | Type | Description |
|------|------|-------------|
| **EnactedSetCodeProposers** | `StorageMap<Blake2_128Concat, AccountId, ()>` | Set of accounts eligible for TC membership (proposers of enacted set_code referenda). Populated by `submit_candidacy`, **add_candidate_force** (Root), and optionally **genesis**. |

### Extrinsic

- **`submit_candidacy(state_root, referendum_index, proposal_hash, set_code_proof)`**  
  - **Origin:** Signed.  
  - **Parameters:**  
    - `state_root`: State root of the block when the referendum was still **Ongoing** (so `ReferendumInfoFor` contained `ReferendumStatus` with proposer and proposal hash).  
    - `referendum_index`: The referendum index (Referenda instance).  
    - `proposal_hash`: The proposal hash (must match the one in the proof and must be a `set_code` call).  
    - `set_code_proof`: A **storage read proof** for `ReferendumInfoFor(referendum_index)` at that block (SCALE-encoded `StorageProof`).  
  - **Logic:**  
    1. Verify the proof against `state_root` and decode the proven value as `ReferendumInfo::Ongoing(ReferendumStatus { submission_deposit, proposal_hash, .. })`.  
    2. Ensure **signer == submission_deposit.who** and **proposal_hash** matches.  
    3. Ensure the referendum is **currently Approved** (current storage).  
    4. Ensure the preimage of `proposal_hash` is **`System::set_code`** (Preimage pallet + decode as `Call`).  
    5. Insert signer into **EnactedSetCodeProposers**.  
  - **Errors:** `InvalidProof`, `NotOngoing`, `NotProposer`, `ProposalHashMismatch`, `ReferendumNotApproved`, `NotSetCode`, `AlreadyCandidate`.

- **`add_candidate_force(who)`**  
  - **Origin:** Root only.  
  - **Parameters:**  
    - `who`: Account to add to the TC candidate list.  
  - **Logic:** Insert `who` into **EnactedSetCodeProposers** without any proof. Use for bootstrapping or recovery when the normal proof path is not available.  
  - **Errors:** `AlreadyCandidate` if `who` is already in the list.

### Genesis

- **GenesisConfig:** `initial_candidates: Vec<AccountId>` — optional list of accounts to seed **EnactedSetCodeProposers** (e.g. bootstrap before any set_code referendum has been enacted).

### Config (runtime)

- **ReferendaPalletName:** Pallet name of the Referenda instance used for the proof (e.g. `"Referenda"` or `"RankedPolls"`).  
- **DecodeOngoing:** Decodes raw referendum storage bytes to `(proposer, proposal_hash)` for the Ongoing variant.  
- **ReferendumApproved:** Checks if a referendum index is currently in Approved state.  
- **SetCodeChecker:** Checks if a proposal hash corresponds to `System::set_code` (Preimage + decode).

---

## RPC: Proof generation

The node exposes RPC methods so clients can build the inputs for `submit_candidacy`.

### 1. `tc_candidates_getReferendumInfoStorageKey(referendum_index: u32) -> Vec<u8>`

Returns the storage key for `ReferendumInfoFor(referendum_index)` (Referenda pallet).  
The client can call the standard **`state_getReadProof([key], block_hash)`** with this key to obtain the proof.

### 2. `tc_candidates_getSetCodeProof(block_hash, referendum_index) -> TcCandidatesProof`

Returns in one call:

- **state_root:** State root of the block (for proof verification).  
- **set_code_proof:** SCALE-encoded `StorageProof` for `ReferendumInfoFor(referendum_index)` at that block.

The client then calls **`TcCandidates::submit_candidacy(state_root, referendum_index, proposal_hash, set_code_proof)`** (with `proposal_hash` from the decoded referendum or from Preimage).

**Flow:**

1. User picks a block when the referendum was still Ongoing and knows the referendum index and proposal hash.  
2. User (or wallet) calls **`tc_candidates_getSetCodeProof(block_hash, referendum_index)`** and gets `{ state_root, set_code_proof }`.  
3. User submits **`submit_candidacy(state_root, referendum_index, proposal_hash, set_code_proof)`** with their account as signer.

---

## TC membership enforcement

A **BaseCallFilter** (or equivalent) in the runtime must restrict:

- **Membership** (`add_member`, `swap_member`, `reset_members`): only allow accounts in **EnactedSetCodeProposers**.  
- **TechnicalCommittee::set_members**: only allow if every new member is in **EnactedSetCodeProposers**.

So the TC candidate list is exactly **EnactedSetCodeProposers**; candidates are added via **submit_candidacy** (proof) or **add_candidate_force** (Root). Optionally the set can be seeded at genesis.

---

## Files

| Piece | Path |
|-------|------|
| Pallet | `pallets/tc-candidates/` |
| RPC | `nodes/poscan-consensus/src/tc_candidates_rpc.rs` |
| Runtime config | `runtime/src/lib.rs` (Config impl, construct_runtime) |
| Referenda helper | `pallets/referenda/src/types.rs` (`ongoing_proposer_and_hash()`) |

---

## Storage key for ReferendumInfoFor

For reference, the key used for proof generation and verification is:

```
twox_128(ReferendaPalletName) ++ twox_128("ReferendumInfoFor") ++ blake2_128_concat(referendum_index.encode())
```

With **ReferendaPalletName** = `"Referenda"` (or `"RankedPolls"` for the second instance).
