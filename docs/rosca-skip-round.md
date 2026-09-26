# ROSCA Round-Skip Mechanism

## Overview

Ahjoor ROSCA lets a member formally request to **skip a contribution round** rather than contribute or default. A skip is a paid absence: the member pays a configured skip fee, and in return the contract treats the round as if they contributed (no default is recorded) and redistributes the payout to the next eligible recipient.

The feature is implemented by `request_skip` in `contracts/ahjoor-rosca/src/lib.rs` and tested in `contracts/ahjoor-rosca/src/test_skip.rs`.

## When `request_skip` Is Allowed

`request_skip(member, round)` enforces the following preconditions before accepting a skip request:

1. **Round is not in the past.** The `round` argument must be ≥ the current round. Requesting a skip for a round that has already passed panics with `RoundDeadlinePassed`.
2. **Contribution window is open.** If `round == current_round`, the current timestamp must be before the round's contribution deadline. A request made after the deadline panics with `ContributionWindowClosed`.
3. **Caller is a group member.** Non-members are rejected with `NotAMember`.
4. **Member has not already contributed this round.** If the member is already in `PaidMembers` for the current round, the call panics with `AlreadyContributed`.
5. **Member has not already skipped this round.** Duplicate skip requests for the same `(member, round)` pair are rejected with `AlreadySkipped`.
6. **Per-cycle skip limit not exceeded.** Each member may skip at most `max_skips_per_cycle` rounds per ROSCA cycle (one full rotation through all members). Exceeding the limit panics with `SkipLimitReached`.

## How the Skip Fee Is Calculated

The skip fee is a flat amount configured at group initialization via `RoscaConfig::skip_fee`. It is denominated in the group's contribution token. When a skip is approved:

- The contract immediately transfers `skip_fee` tokens from the member to the contract address.
- If `skip_fee` is `0`, no transfer is made and the skip is free.

The skip fee is then included in the current round's pot alongside the contributions of the other members, so the payout recipient receives a slightly larger pot (normal contributions + skip fee).

## Effect on Payout Order and Default Tracking

After a successful `request_skip`:

- **Payout order.** The skipping member is removed from contention as the current round's recipient. If the skipping member was the scheduled recipient, the payout moves to the next member in the payout order.
- **Default tracking.** No default is recorded for the skipping member. The member's `default_count` remains unchanged, and they are not penalized beyond the skip fee.
- **Next round.** The skipping member participates normally in subsequent rounds. Skipping one round does not shift their position in the overall payout order for future cycles.

Example from the test suite:

```
Members: [A, B, C]  |  contrib_amount: 100  |  skip_fee: 50  |  max_skips_per_cycle: 1

Round 0 (scheduled recipient: A):
  A requests_skip → pays 50 fee, skip recorded
  B contributes 100
  C contributes 100
  Pot = 100 + 100 + 50 = 250 → paid to B (next eligible)
  A: default_count unchanged, balance = 950

Round advances to 1.
```

## Per-Cycle Skip Limit

`max_skips_per_cycle` is set via `RoscaConfig` at initialization and is applied per member per ROSCA cycle. A *cycle* is one complete rotation through all payout slots (`payout_order.len()` rounds). The cycle index is computed as `round / payout_order.len()`.

Once a member has used all their skip allowances for a cycle, further `request_skip` calls fail with `SkipLimitReached` until the next cycle begins.

## Summary

| Condition | Behavior |
|---|---|
| Called after contribution deadline | Panics `ContributionWindowClosed` |
| Member already contributed | Panics `AlreadyContributed` |
| Member already skipped this round | Panics `AlreadySkipped` |
| Skip limit reached for cycle | Panics `SkipLimitReached` |
| `skip_fee == 0` | Skip is granted for free |
| `skip_fee > 0` | Fee transferred to contract, added to pot |
| Skipping member was payout recipient | Payout shifts to next eligible member |
| Default tracking | No default recorded for skipping member |

## Related Documentation

- [Contribution Receipts in ROSCA](rosca-contribution-receipts.md)
- [ROSCA Reinvestment Flow](rosca-reinvestment.md)
- [On-Chain Audit Trail in ROSCA](rosca-audit-trail.md)
