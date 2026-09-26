# Multi-Token Invoice in ahjoor-payments

## Overview

ahjoor-payments supports invoices that can be paid in more than one token. A merchant creates a single invoice denominated in a base currency, lists which tokens they accept, and lets the customer pay in any combination of those tokens. The contract converts each token payment to the base currency using a per-token conversion rate (set by the merchant or resolved through an on-chain oracle) and tracks partial payments until the invoice is fully covered. Completed invoices can then be batch-settled into a single record.

The feature is implemented in `contracts/ahjoor-payments/src/multi_token_invoice_impl.rs` and the supporting types in `contracts/ahjoor-payments/src/multi_token_invoice.rs`.

## Creating an invoice

### `create_invoice`

```rust
create_invoice(
    env, merchant, customer,
    total_amount, base_currency,
    accepted_tokens,          // Vec<Address>
    preferred_settlement_token,
    line_items,               // Vec<InvoiceLineItem>, max 20
    due_date,
    metadata,
)
```

A convenience wrapper that delegates to `create_invoice_with_oracle` with `oracle_contract = None`. The caller (merchant) must authorize the call. Returns a monotonically incrementing invoice ID.

Validation: `total_amount > 0`, at most 20 line items, each item requires `quantity > 0` and `unit_price > 0`.

### `create_invoice_with_oracle`

```rust
create_invoice_with_oracle(
    ...,
    oracle_contract: Option<Address>,
)
```

Identical to `create_invoice` but optionally records an on-chain oracle contract address on the invoice. When an oracle is present, customers can pay via `pay_invoice_cross_token` and have the rate resolved at payment time rather than relying on a pre-set conversion rate.

The invoice is stored under a `(Symbol("invoice"), invoice_id)` persistent key. Initial `settlement_conversion_rate` defaults to `1_000_000` (i.e., 1:1).

### Setting per-token conversion rates

Before payments can be accepted, conversion rates must be configured. Rates are scaled by `1_000_000` (one unit of payment token = `rate / 1_000_000` units of base currency):

```rust
// Merchant-wide rate (applies to all invoices for this merchant)
set_conversion_rate(env, merchant, token, rate_to_base);

// Per-invoice settlement rate (base → settlement token)
set_settlement_conversion_rate(env, merchant, invoice_id, rate);
```

`accept_payment` looks for an invoice-specific rate first, then falls back to the merchant-wide rate. If no rate is found for the payment token, the call panics with `ConversionRateNotSet`.

## How partial payments are tracked

```rust
accept_payment(env, invoice_id, payer, token, amount) -> InvoicePayment
```

1. Validates that the invoice exists and is not `Cancelled` or `FullyPaid`.
2. Confirms the token is in `accepted_tokens`.
3. Resolves the conversion rate and computes `amount_in_base = amount * rate / 1_000_000`.
4. Converts to settlement units: `amount_in_settlement = amount_in_base * settlement_conversion_rate / 1_000_000`.
5. Adds `amount_in_base` to the per-token running total stored in `invoice.payments_received`.
6. Sums all token totals to get the cross-token aggregate; if that aggregate equals or exceeds `total_amount`, the invoice transitions to `FullyPaid`; otherwise it becomes `PartiallyPaid`.
7. Stores an `InvoicePayment` record and indexes it under the invoice for retrieval via `get_invoice_payments`.

Overpayment is rejected: the call panics with `PaymentExceedsInvoiceAmount` if the new aggregate would exceed `total_amount`.

### Oracle-assisted cross-token payment

```rust
pay_invoice_cross_token(
    env, invoice_id, payer,
    payment_token, payment_amount,
    max_slippage_bps,
)
```

Requires the invoice to have an `oracle_contract`. The oracle's `get_price(payment_token, base_currency)` is called to get a live rate. If the oracle price deviates from the invoice's stored conversion rate by more than `max_slippage_bps` basis points, the call is rejected (`SlippageExceeded`). On success, `payment_token` is transferred from `payer` to the contract and a `CrossTokenSettlement` event is emitted.

## What `settle_invoices` does

```rust
settle_invoices(env, merchant, invoice_ids) -> SettlementBatch
```

Merchants call this to close a batch of up to 50 fully-paid invoices. The call:

1. Verifies the caller is the merchant for every invoice in `invoice_ids`.
2. Requires every invoice to be in `FullyPaid` status; any other status panics with `InvalidInvoiceStatus`.
3. Sums the `total_amount` of all invoices into `total_settlement_amount`.
4. Creates and stores a `SettlementBatch` record (status `Completed`, `settled_at` timestamp, list of invoice IDs).

The batch record can be retrieved later with `get_settlement_batch(env, batch_id)`.

## Storage keys

- `(Symbol("invoice"), invoice_id)` — persistent `MultiTokenInvoice` record.
- `(Symbol("inv_payment"), payment_id)` — persistent `InvoicePayment` record.
- `(Symbol("inv_pay_idx"), invoice_id)` — persistent index of payment IDs for an invoice.
- `(Symbol("settle_batch"), batch_id)` — persistent `SettlementBatch` record.
- `(Symbol("merch_rates"), merchant)` — persistent map of token → rate for a merchant.
- `Symbol("invoice_counter")` / `Symbol("payment_counter")` / `Symbol("settlement_batch_counter")` — instance-level ID counters.

## Test coverage

Tests are inline in `contracts/ahjoor-payments/src/multi_token_invoice_impl.rs`:

- Multiple installments across different tokens, cumulative status changes, and final `FullyPaid` detection.
- `get_invoice_payments` returns an empty list for an unpaid invoice.
