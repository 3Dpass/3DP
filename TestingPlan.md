# Testing Plan

TODO:

---

## A. Development Mode

### 1. TC Candidates pallet (`pallets/tc-candidates`)

#### Overview

The **TC Candidates** pallet (`pallet_tc_candidates`) restricts Technical Committee (TC) membership to accounts that have been the **proposer** of an **enacted** runtime-upgrade referendum (`System::set_code`). The list of eligible accounts (`EnactedSetCodeProposers`) is the TC candidate list: only this list is checked when adding TC members (via a runtime call filter). Candidates join either by calling **`submit_candidacy`** with a **state proof** that at a past block they were the proposer of a referendum that is now Approved and whose proposal was `set_code`, or by **Root** via **`add_candidate_force`** (bypasses the proof; for bootstrapping or recovery).

#### Technical Committee members rotation test

- A user submits a Referenda **set_code** proposal → Approve → runtime upgrade enacted.
- The proposer generates a proof by calling **`tc_candidates_getSetCodeProof(block_hash, referendum_index)`** and obtains `{ state_root, set_code_proof }`.
- Submit the proposer’s candidacy by calling **`submit_candidacy(state_root, referendum_index, proposal_hash, set_code_proof)`** with their account as the signer.
- Verify the TC candidate is added to the list.
- Add one more TC candidate via **`add_candidate_force`** (Root); verify the account was added.
- Council: pass a new motion to add the TC candidate (51% Aye required to pass).
- Council: pass a new motion to remove the TC candidate (51% Aye required to pass).
- Council: pass a new motion to swap the TC candidate (51% Aye required to pass).
- Council: pass a new motion to remove the TC candidate (51% Aye required to pass).

### 2. Vested transfer to Treasury

- **Create the vesting on Treasury:**
  - **Call:** `Vesting::vested_transfer(target, schedule)`
  - **Target:** `Treasury::account_id()` (the Treasury account).
  - **Schedule:** `VestingInfo { locked, per_block, starting_block }` – total locked amount, unlock rate per block, and start block for linear vesting.

- **Unlock over time (update the lock)** – anyone can trigger the unlock.

  The vesting pallet does **not** auto-update locks. As time passes, the *vested* amount increases by the schedule, but the on-chain lock is only reduced when someone calls:

  - **Call:** `Vesting::vest_other(target)`
  - **Target:** `Treasury::account_id()`.

### 3. **withdraw_assets** function test (EVM pallet)

- Create a fungible asset.
- Transfer it into the EVM.
- Transfer it back to the Substrate-mapped account.
- Call **withdraw_assets** to complete the transfer.

---

## B. Full Testnet (public upgrade testing)

1. Run the full testnet from the latest mainnet commit `9de7f4a`.
2. Build the latest “sudo-free” runtime (WASM).
3. Submit the upgrade.

---

## C. Mainnet upgrade

1. Merge into `main`.
2. Build the sudo-free release on GitHub.
3. Execute the upgrade online.
