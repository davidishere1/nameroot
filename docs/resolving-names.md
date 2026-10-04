# Resolving names in your app

## Forward lookup (name → address)

Simulate `resolve(name)`. It's read-only, so no wallet or fee is needed. Treat
`NameExpired (6)` and `NameNotFound (5)` as "no such recipient" and never
fall back to a cached address for an expired name.

## Showing names instead of keys

Simulate `primary_name(address)`. `None` means show the shortened key.

## Before sending a payment

1. Lowercase and trim the user's input, and reject anything outside `[a-z0-9-]`.
2. Resolve it and show the resulting address to the user.
3. Resolve again right before signing, in case the name changed hands.

## Pricing guidance

`price_per_year` is in the fee token's base units. Something like 5 XLM a
year is enough to make bulk squatting expensive while staying trivial for
real users.
