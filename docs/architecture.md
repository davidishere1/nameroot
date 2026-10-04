# Architecture

## Records

```text
Name(String) → Record { owner, target, expires_at }
Primary(Address) → String          // reverse record (address → name)
Settings { admin, fee_token, price_per_year, treasury }
```

`owner` controls the name and `target` is what it resolves to. Keeping them
separate allows cold-wallet ownership with a hot-wallet target.

## Lifecycle

```text
register ──▶ active ──(expires_at)──▶ grace (30d) ──▶ available
               ▲                       │
               └──── renew ◀───────────┘
```

- **Active**: resolves, and the owner can retarget or transfer.
- **Grace**: does *not* resolve, can't be registered by others, can be renewed.
- **Available**: anyone may register it again.

## Name rules

3–32 bytes of `a–z`, `0–9` and inner `-`. Lowercase-only plus ASCII-only
rules out case collisions and homoglyph look-alikes (`аlice` with a
Cyrillic `а`).

## Reverse records can't go stale

`primary_name(address)` re-checks on every read that the name is active
and still targets `address`. Retargeting or expiry invalidates the reverse
record without a separate cleanup step.
