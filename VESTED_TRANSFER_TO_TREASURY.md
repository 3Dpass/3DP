# Vested Transfer to Treasury

This document describes how to transfer funds to the Treasury account with a vesting schedule, so that the amount unlocks over time according to the schedule and can be spent by Council via normal Treasury proposals.

## Overview

The runtime includes:

- **Vesting** (`pallet_vesting`) – manages locked balances with a linear unlock schedule.
- **Treasury** (`pallet_treasury`) – holds funds at `Treasury::account_id()` and spends them via Council approval.

You can create a vesting schedule on the Treasury account. Funds are locked until they vest; once the lock is updated (see below), the unlocked portion becomes normal Treasury balance and can be spent by Council.

## Flow

### 1. Create the vesting on Treasury

Choose one of:

**Option A – Signed vested transfer**

Any account with sufficient balance can call:

- **Call:** `Vesting::vested_transfer(target, schedule)`
- **Target:** `Treasury::account_id()` (the Treasury account).
- **Schedule:** `VestingInfo { locked, per_block, starting_block }` – total locked amount, unlock rate per block, and start block for linear vesting.

Funds are transferred from the signer to the Treasury account and locked according to the schedule. Minimum transfer is `MinVestedTransfer` (100 CENTS in this runtime; see `runtime/src/lib.rs` around line 1266).

**Option B – Force vested transfer (root/sudo)**

Root (or sudo) can call:

- **Call:** `Vesting::force_vested_transfer(source, target, schedule)`
- **Source:** Any account whose balance will be debited.
- **Target:** `Treasury::account_id()`.
- **Schedule:** Same `VestingInfo` as above.

Use this when the funds should come from an account that is not the signer (e.g. a system or foundation account).

### 2. Unlock over time (update the lock)

The vesting pallet does **not** auto-update locks. As time passes, the *vested* amount increases by the schedule, but the on-chain lock is only reduced when someone calls:

- **Call:** `Vesting::vest_other(target)`
- **Target:** `Treasury::account_id()`.

This recalculates the lock for the Treasury account and sets it to the remaining unvested amount. The newly vested amount becomes free balance.

**Who can call:** `vest_other` requires only a **Signed** origin, so any account (including a Council member) can submit it. You can run it as a Council motion if you want it to be “by Council vote,” or have a script/bot call it periodically.

**When to call:** Call it whenever you want to reflect the current block in the lock (e.g. before a Treasury spend, or on a regular schedule). If you never call it, the lock never shrinks and the vested amount stays non-spendable.

### 3. Spend the unlocked amount

After `vest_other(Treasury::account_id())` has been called, the unlocked portion is part of Treasury’s **free** balance. Council can then:

- Use the usual Treasury flow: propose spend → approve (e.g. 3/5 Council) → spend.
- The spend transfers from `Treasury::account_id()` to the beneficiary; only the free (unlocked) balance can be used.

## Summary

| Step | Action | Origin |
|------|--------|--------|
| 1 | Create vesting on Treasury | `vested_transfer` (signed) or `force_vested_transfer` (root) |
| 2 | Update lock so vested amount is free | `vest_other(Treasury::account_id())` (any signed) |
| 3 | Spend unlocked funds | Normal Treasury proposals (Council) |

## Notes

- **Schedule is fixed:** Vesting is linear and time-based (by block). Council cannot vote to unlock earlier or change the curve; they can only trigger the lock update so it matches the schedule at the current block.
- **Runtime config:** Vesting is configured in `runtime/src/lib.rs` (e.g. `impl pallet_vesting::Config`, `MinVestedTransfer`, `MAX_VESTING_SCHEDULES`). Treasury account ID is used in several places (e.g. `DonationDestination`, `TreasuryPalletId`).
- **Reference:** Substrate vesting pallet: [frame/vesting](https://github.com/paritytech/substrate/tree/master/frame/vesting).
