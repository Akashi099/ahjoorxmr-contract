# ROSCA Reinvestment Flow

## Overview

Ahjoor ROSCA lets any group member opt in to automatic **payout reinvestment**. When reinvestment is enabled for a member who is the current round's recipient, the payout is not transferred to their wallet. Instead the full pot amount is credited toward their contribution for the *next* round, effectively rolling the payout forward and keeping the funds inside the group.

The feature is controlled by two entry points in `contracts/ahjoor-rosca/src/lib.rs`:

- `set_reinvest_preference(member, reinvest: bool)` — enables or disables automatic reinvestment for the calling member.
- `get_reinvest_preference(member) -> bool` — returns the member's current reinvestment preference (`true` = reinvest, `false` = receive payout, default `false`).

Behaviour is verified by `contracts/ahjoor-rosca/src/test_reinvest.rs`.

## What Setting the Reinvest Preference Changes

When a member's reinvestment preference is `true` and they are selected as the payout recipient at round finalization:

1. **No outbound transfer.** The contract does not call `transfer` to send the pot to the recipient's wallet.
2. **Credit applied to next round.** The full pot amount is recorded as the member's contribution for the *next* round (`paid_members` is updated and the contributed amount is set to the pot total).
3. **Over-payment handling.** If the pot exceeds the required contribution amount for the next round, the surplus is carried as a negative `remaining` balance (i.e., the member is "pre-paid" beyond the required amount). If the pot is less than the required contribution, the member must top up the difference before the next round's deadline.
4. **Default tracking unaffected.** Because the reinvested amount counts as a contribution, the member does not accrue a default for the next round.

When the preference is `false` (the default), payout handling is unchanged: the pot is transferred to the recipient's wallet as normal.

## When the Preference Can Be Toggled

`set_reinvest_preference` enforces a **deadline constraint**: it may only be called before the current round's contribution deadline has elapsed. Attempting to set the preference after the deadline panics with `ContributionWindowClosed` (error `#33`).

The preference can be toggled freely — from `false` to `true`, or back — any number of times as long as the deadline has not passed. There is no minimum holding period and no fee for toggling. The preference that is active at the moment the round is finalized is the one that takes effect.

Typical usage pattern:

```rust
// Enable reinvestment before the round deadline
client.set_reinvest_preference(&member, &true);

// Confirm the setting
assert!(client.get_reinvest_preference(&member));

// Disable reinvestment again before the deadline
client.set_reinvest_preference(&member, &false);
assert!(!client.get_reinvest_preference(&member));
```

## Summary

| Function | Auth | Constraint |
|---|---|---|
| `set_reinvest_preference(member, reinvest)` | member | Before round contribution deadline |
| `get_reinvest_preference(member) -> bool` | none | Read-only, returns `false` if never set |

## Related Documentation

- [Contribution Receipts in ROSCA](rosca-contribution-receipts.md)
- [On-Chain Audit Trail in ROSCA](rosca-audit-trail.md)
