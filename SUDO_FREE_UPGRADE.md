# Going Sudo-Free via Runtime Upgrade

## Inspection Summary

### Current Sudo Usage

1. **Runtime (`runtime/src/lib.rs`)**
   - `impl pallet_sudo::Config for Runtime` (lines 1506–1509)
   - `Sudo: pallet_sudo` in `construct_runtime!` (line 1792)

2. **Genesis (`runtime/src/genesis.rs`)**
   - `SudoConfig { key: Some(root_key) }` — root key set at chain genesis

3. **No other code** in the repo dispatches `Call::Sudo` or depends on the Sudo pallet.  
   Comments in `pallets/validator-set` mentioning "sudo/root" refer to governance concepts, not the pallet.

### Is the Sudo Pallet being removed entirely?

**No.** We do **not** need to remove the pallet from the runtime.

- **Removing the pallet** from `construct_runtime!` would:
  - Change the `Call` enum (pallet indices for every pallet after Sudo).
  - Break decoding of existing extrinsics and break compatibility with clients/JS that rely on call indices.
- **Recommended approach:** keep the Sudo pallet and **clear the Sudo key** in a one-time migration. After that:
  - No account can call `Sudo::sudo`, `Sudo::set_key`, or `Sudo::sudo_as`.
  - The pallet remains in the runtime but is effectively unused (no key → `RequireSudo` on all calls).

So: **we will keep the pallet, remove the sudo key via migration.**

### What Else Is There to Do?

1. **Runtime migration (done)**  
   Add an `OnRuntimeUpgrade` that clears the Sudo `Key` storage (see implementation below).  
   This migration runs once when the new runtime is applied.  
   **Upgrade flow:** current sudo key submits `Sudo::sudo(System::set_code(new_wasm))` (or equivalent) so the new runtime (with the migration) is applied; on first block of the new runtime, the migration runs and clears the key.

2. **Genesis (optional)**  
   For **new chains** (forks) created after this change, anyone can set `sudo: SudoConfig { key: None }` so they start without a sudo key. Existing chain is updated by the migration only.

3. **EnsureRoot elsewhere**  
   Many pallets (Scheduler, Preimage, Referenda, Democracy, etc.) use `EnsureRoot<AccountId>`. That is **frame_system::RawOrigin::Root**, not the Sudo pallet. Clearing the Sudo key only removes the ability for any account to *produce* Root via Sudo; we do **not** need to change those `EnsureRoot` configs.

---

## Implementation

- **Migration:** `RemoveSudoKey` in `runtime/src/lib.rs` clears the Sudo pallet’s `Key` storage using the same key layout as FRAME (`twox_128("Sudo") ++ twox_128("Key")`). It is registered in the Executive’s migration list and runs once when the new runtime is applied.
- **Genesis:** For **new** chains (forks) created after this change, anyone can set `sudo: SudoConfig { key: None }` in `runtime/src/genesis.rs` so they start without a sudo key. Existing chains are updated only by the migration.

After the upgrade, the chain is “sudo-free”: no key is set, so no further sudo calls are possible.
