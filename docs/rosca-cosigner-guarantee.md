# Co-Signer Guarantee in ahjoor-rosca

## Overview

A member of an `ahjoor-rosca` group can designate a trusted co-signer who is authorized to cover a missed contribution on that member's behalf. If the member defaults at the end of a round and a grace window is configured, the contract opens a ledger-bounded window during which the co-signer can step in. A contribution made through `co_signer_contribute` is recorded as the member's own, preventing the default penalty from being applied. If no one acts before the window closes, the penalty is applied as normal and the window is cleared.

The feature is implemented in `contracts/ahjoor-rosca/src/lib.rs` and covered by `contracts/ahjoor-rosca/src/test_cosigner_guarantee.rs`.

## Nominating a co-signer

```rust
set_co_signer(env, member, group_id, co_signer)
```

The member authorizes the call. The contract verifies that the caller is a current group member and that no co-signer has already been set for this member (each member can have at most one co-signer at a time). A `CoSignerRecord` is created with `status = Pending` and stored in the shared `CoSigners` map keyed by the member's address. A `CoSignerSet` event is emitted.

## Accepting the designation

```rust
accept_co_signer(env, co_signer, group_id, member)
```

The nominated co-signer authorizes this call. The contract looks up the `CoSignerRecord` for `member`, confirms that `record.co_signer == co_signer`, and upgrades the status from `Pending` to `Active`. A `CoSignerAccepted` event is emitted.

A co-signer that remains in `Pending` status is treated as unaccepted: during default processing, a pending co-signer is ignored and the penalty is applied immediately without opening a grace window.

## What `co_signer_contribute` does

```rust
co_signer_contribute(env, co_signer, group_id, member, token, amount)
```

The co-signer authorizes the call. Before any token movement, the contract checks:

1. A `CoSignerRecord` exists for `member` with `record.co_signer == co_signer` and `status == Active`.
2. A grace window is currently open for `member` (i.e., `CoSignerWindowStart` contains an entry for `member`).
3. The window has not expired: `current_ledger < window_start + co_signer_window_ledgers`.

If all checks pass, `amount` of `token` is transferred from the co-signer to the contract and the contribution is recorded under the member's name (the member is added to `PaidMembers`). The open window entry for the member is then cleared, closing the grace period. A `CoSignerContributed` event is emitted.

## How the grace window works

### Configuring the window

```rust
set_co_signer_window(env, admin, window_ledgers)
```

Only the admin can call this. `window_ledgers` is the number of ledgers a co-signer has to act after the window is opened. Setting `window_ledgers = 0` effectively disables grace windows: the co-signer mechanism will still record the designation but no window will ever be opened and the penalty path is unchanged.

```rust
get_co_signer_window(env) -> u32
```

Returns the current setting (defaults to `0` if unset).

### Window lifecycle during default processing

When the round finalizes and the contract processes defaulting members:

1. If a defaulting member has an `Active` co-signer and `window_ledgers > 0`, the contract opens a window by recording the current ledger sequence in `CoSignerWindowStart` and skips the penalty for that member.
2. On subsequent finalization calls, if the window is still open (not yet expired), the penalty continues to be skipped.
3. If the window has expired (co-signer did not act in time), the entry is cleared, a `CoSignerWindowExpired` event is emitted, and the penalty is applied normally.

### Removing a co-signer designation

```rust
remove_co_signer(env, member, group_id)
```

The member authorizes the call. Removal is only allowed between rounds: the call panics with `CannotChangeMidRound` if `PaidMembers` is non-empty. On success, the `CoSignerRecord` for the member is deleted from the shared map.

## Querying co-signer state

```rust
get_co_signer_record(env, group_id, member) -> Option<CoSignerRecord>
```

Returns the `CoSignerRecord` for `member`, or `None` if no co-signer has been set. The record contains the co-signer address and current status (`Pending` or `Active`).

## Storage keys

- `DataKey4::CoSigners` — instance-level `Map<Address, CoSignerRecord>` containing all active designations.
- `DataKey3::CoSignerWindowStart` — instance-level `Map<Address, u32>` tracking when each member's grace window was opened (ledger sequence).
- `DataKey4::CoSignerWindowLedgers` — instance-level global window duration in ledgers.

## Events

- `CoSignerSet { group_id, member, co_signer }` — emitted when a member nominates a co-signer.
- `CoSignerAccepted { group_id, member, co_signer }` — emitted when the co-signer accepts.
- `CoSignerContributed { group_id, member, co_signer, amount }` — emitted when the co-signer covers a contribution.
- `CoSignerWindowExpired { group_id, member }` — emitted when the grace window closes without the co-signer acting.

## Test coverage

Tests are in `contracts/ahjoor-rosca/src/test_cosigner_guarantee.rs`:

- Co-signer successfully honours a missed contribution within the window.
- Window expiry triggers the member penalty.
- `remove_co_signer` correctly clears the designation.
- A co-signer that has not called `accept_co_signer` cannot contribute.
- A pending co-signer is skipped during default processing and the penalty is applied immediately.
