# Tipping in ahjoor-payments

## Overview

ahjoor-payments supports optional customer-to-merchant gratuities on top of a base payment. A payment must be explicitly created with tipping enabled, and the tip can only be collected at settlement time. The contract enforces a global basis-point cap on tip size, and merchants can configure a split that distributes the tip across multiple recipient addresses instead of routing the full amount to the merchant directly.

The feature is implemented in `contracts/ahjoor-payments/src/lib.rs` and covered by `contracts/ahjoor-payments/src/test_tip_subscription.rs`.

## Attaching a tip to a payment

### Step 1 — create a tipping-enabled payment

```rust
create_payment_with_tipping(
    env, customer, merchant, amount, token,
    reference, metadata, idempotency_key,
) -> u32
```

This function creates a standard payment and then sets `tipping_enabled = true` on the stored record. Any payment created through the regular `create_payment` or `create_payment_with_expiry` paths has `tipping_enabled = false` and cannot accept a tip.

### Step 2 — settle with a tip

```rust
complete_payment_with_tip(env, payment_id, customer, tip_amount)
```

The admin authorizes the call. When `tip_amount == 0` the function behaves identically to `complete_payment` and the customer authorization is not required. When `tip_amount > 0`:

1. Verifies `payment.tipping_enabled == true`; panics with `TippingNotEnabled` otherwise.
2. Reads the global `MaxTipBps` cap (default `3000` = 30%) and computes `max_tip = payment.amount * max_tip_bps / 10_000`.
3. Rejects the call with `TipExceedsMaxBps` if `tip_amount > max_tip`.
4. Requires the customer to authorize (to authorize the token pull).
5. Distributes the tip — see the split section below.
6. Calls `complete_payment_internal` to finalize the base payment.

## How `set_max_tip_bps` caps the tip

```rust
set_max_tip_bps(env, admin, max_bps)
```

Only the admin can call this. `max_bps` must be ≤ `10_000` (100%); values above that panic. The default is `3_000` (30%). The cap is enforced per-payment at settlement time using the base `payment.amount`, so a 30% cap on a 100-token payment allows a maximum tip of 30 tokens.

```rust
get_max_tip_bps(env) -> u32
```

Returns the current cap.

## How `set_tip_split_config` divides the tip

By default, when no split is configured, the entire tip is transferred directly from the customer to the merchant. Merchants can override this with a named list of beneficiaries:

```rust
set_tip_split_config(
    env,
    merchant,
    caller,                        // must equal merchant
    split_map: Vec<(Address, u32)> // (recipient, bps)
)
```

Validation:
- `caller` must equal `merchant`.
- `split_map` must be non-empty.
- The number of entries must not exceed `MAX_TIP_SPLIT_BENEFICIARIES`.
- All `bps` values must sum to exactly `10_000`.

At settlement, when a split config exists, `complete_payment_with_tip` iterates over beneficiaries and transfers `tip_amount * bps / 10_000` to each recipient. Zero-amount allocations (rounding edge cases) are silently skipped. A `TipSplit` event is emitted per beneficiary.

When no split is configured the entire tip is sent to the merchant and a single `TipReceived` event is emitted.

### Removing a split config

```rust
remove_tip_split_config(env, merchant, caller)
```

`caller` must equal `merchant`. Removes the stored config; future tips revert to 100% to merchant.

### Reading the current config

```rust
get_tip_split_config(env, merchant) -> Option<Vec<(Address, u32)>>
```

Returns the configured beneficiary list, or `None` if no config exists.

## Storage keys

- `DataKey::Payment(u32)` — persistent payment record; the `tipping_enabled` flag is part of this record.
- `DataKey2::MaxTipBps` — instance-level global tip cap.
- `DataKey3::TipSplitConfig(Address)` — persistent per-merchant split configuration.

## Events

- `TipReceived { payment_id, recipient, amount, token }` — emitted when the full tip goes to the merchant.
- `TipSplit { payment_id, recipient, amount, token }` — emitted once per beneficiary when a split config distributes the tip.

## Test coverage

Tests are in `contracts/ahjoor-payments/src/test_tip_subscription.rs`:

- Interval enforcement on tip submissions.
- Cancellation stops further tip execution.
- Third-party relayer execution of a tip.
- Tip subscription behaviour independent of the invoice payment model.
