#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token::StellarAssetClient,
    Env, String,
};

const NOW: u64 = 1_700_000_000;
const PRICE: i128 = 50_000_000; // 5 units/year

struct Setup<'a> {
    env: Env,
    names: NamerootClient<'a>,
    token_client: token::Client<'a>,
    treasury: Address,
    alice: Address,
    bob: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = NOW);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let treasury = Address::generate(&env);
    let names = NamerootClient::new(
        &env,
        &env.register(
            Nameroot,
            (
                Address::generate(&env),
                token.clone(),
                PRICE,
                treasury.clone(),
            ),
        ),
    );
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let mint = StellarAssetClient::new(&env, &token);
    mint.mint(&alice, &(PRICE * 100));
    mint.mint(&bob, &(PRICE * 100));
    let token_client = token::Client::new(&env, &token);
    Setup {
        env,
        names,
        token_client,
        treasury,
        alice,
        bob,
    }
}

fn n(s: &Setup, name: &str) -> String {
    String::from_str(&s.env, name)
}

fn at(s: &Setup, t: u64) {
    s.env.ledger().with_mut(|l| l.timestamp = t);
}

#[test]
fn registers_resolves_and_charges_the_fee() {
    let s = setup();
    let expires = s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &2);

    assert_eq!(expires, NOW + 2 * YEAR);
    assert_eq!(s.names.resolve(&n(&s, "alice")), s.alice);
    assert_eq!(s.token_client.balance(&s.treasury), 2 * PRICE);
    assert!(!s.names.is_available(&n(&s, "alice")));
}

#[test]
fn validates_names() {
    let s = setup();
    for bad in [
        "ab",
        "Alice",
        "al ice",
        "-alice",
        "alice-",
        "alice_1",
        "ålice",
        &"a".repeat(33),
    ] {
        assert_eq!(
            s.names.try_register(&s.alice, &n(&s, bad), &s.alice, &1),
            Err(Ok(Error::InvalidName)),
            "{bad} should be rejected"
        );
    }
    for good in ["abc", "alice-01", "7eleven", &"z".repeat(32)] {
        assert!(s
            .names
            .try_register(&s.alice, &n(&s, good), &s.alice, &1)
            .is_ok());
    }
}

#[test]
fn rejects_bad_durations() {
    let s = setup();
    assert_eq!(
        s.names
            .try_register(&s.alice, &n(&s, "alice"), &s.alice, &0),
        Err(Ok(Error::InvalidYears))
    );
    assert_eq!(
        s.names
            .try_register(&s.alice, &n(&s, "alice"), &s.alice, &(MAX_YEARS + 1)),
        Err(Ok(Error::InvalidYears))
    );
}

#[test]
fn a_taken_name_cannot_be_registered_again() {
    let s = setup();
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);
    assert_eq!(
        s.names.try_register(&s.bob, &n(&s, "alice"), &s.bob, &1),
        Err(Ok(Error::NameTaken))
    );
}

#[test]
fn expired_names_stop_resolving_and_are_reserved_during_grace() {
    let s = setup();
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);

    at(&s, NOW + YEAR);
    assert_eq!(
        s.names.try_resolve(&n(&s, "alice")),
        Err(Ok(Error::NameExpired))
    );
    // Grace period: bob can't grab it yet.
    assert_eq!(
        s.names.try_register(&s.bob, &n(&s, "alice"), &s.bob, &1),
        Err(Ok(Error::NameTaken))
    );

    // After grace, it's up for grabs.
    at(&s, NOW + YEAR + GRACE_PERIOD);
    assert!(s.names.is_available(&n(&s, "alice")));
    s.names.register(&s.bob, &n(&s, "alice"), &s.bob, &1);
    assert_eq!(s.names.resolve(&n(&s, "alice")), s.bob);
}

#[test]
fn renewing_during_grace_restores_the_name() {
    let s = setup();
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);
    at(&s, NOW + YEAR + 10 * 86_400);

    let expires = s.names.renew(&s.alice, &n(&s, "alice"), &1);
    assert_eq!(expires, NOW + YEAR + 10 * 86_400 + YEAR);
    assert_eq!(s.names.resolve(&n(&s, "alice")), s.alice);
}

#[test]
fn renewing_an_active_name_extends_from_its_expiry() {
    let s = setup();
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);
    // Anyone can pay to renew; ownership stays with alice.
    let expires = s.names.renew(&s.bob, &n(&s, "alice"), &2);
    assert_eq!(expires, NOW + 3 * YEAR);
    assert_eq!(s.names.get_record(&n(&s, "alice")).owner, s.alice);
}

#[test]
fn renewal_is_capped_at_ten_years_out() {
    let s = setup();
    s.names
        .register(&s.alice, &n(&s, "alice"), &s.alice, &MAX_YEARS);
    let expires = s.names.renew(&s.alice, &n(&s, "alice"), &5);
    assert_eq!(expires, NOW + MAX_YEARS as u64 * YEAR);
}

#[test]
fn names_past_grace_cannot_be_renewed() {
    let s = setup();
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);
    at(&s, NOW + YEAR + GRACE_PERIOD);
    assert_eq!(
        s.names.try_renew(&s.alice, &n(&s, "alice"), &1),
        Err(Ok(Error::NameExpired))
    );
}

#[test]
fn owner_can_retarget_and_transfer() {
    let s = setup();
    let hot_wallet = Address::generate(&s.env);
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);

    s.names.set_target(&n(&s, "alice"), &hot_wallet);
    assert_eq!(s.names.resolve(&n(&s, "alice")), hot_wallet);

    s.names.transfer(&n(&s, "alice"), &s.bob);
    let record = s.names.get_record(&n(&s, "alice"));
    assert_eq!(record.owner, s.bob);
    // The name now pays its new owner, not the previous owner's hot wallet.
    assert_eq!(record.target, s.bob);
    assert_eq!(s.names.resolve(&n(&s, "alice")), s.bob);
}

#[test]
#[should_panic]
fn only_the_owner_can_retarget() {
    let s = setup();
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);
    s.env.set_auths(&[]);
    s.names.set_target(&n(&s, "alice"), &s.bob);
}

#[test]
fn reverse_lookup_only_while_the_name_points_back() {
    let s = setup();
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);
    s.names.set_primary(&s.alice, &n(&s, "alice"));
    assert_eq!(s.names.primary_name(&s.alice), Some(n(&s, "alice")));

    // Bob can't claim a name that doesn't point at him.
    assert_eq!(
        s.names.try_set_primary(&s.bob, &n(&s, "alice")),
        Err(Ok(Error::PrimaryMismatch))
    );

    // Retargeting away invalidates the reverse record automatically.
    s.names.set_target(&n(&s, "alice"), &s.bob);
    assert_eq!(s.names.primary_name(&s.alice), None);

    // So does expiry.
    s.names.set_primary(&s.bob, &n(&s, "alice"));
    at(&s, NOW + YEAR);
    assert_eq!(s.names.primary_name(&s.bob), None);
}

#[test]
fn admin_can_change_the_price_and_free_registration_works() {
    let s = setup();
    s.names.set_price(&0);
    s.names.register(&s.alice, &n(&s, "free"), &s.alice, &1);
    assert_eq!(s.token_client.balance(&s.treasury), 0);
    assert_eq!(s.names.try_set_price(&-1), Err(Ok(Error::InvalidPrice)));
}

#[test]
#[should_panic]
fn constructor_rejects_a_negative_price() {
    let env = Env::default();
    let a = Address::generate(&env);
    env.register(Nameroot, (a.clone(), a.clone(), -1i128, a));
}

#[test]
fn admin_and_treasury_can_be_handed_over() {
    let s = setup();
    let new_admin = Address::generate(&s.env);
    let new_treasury = Address::generate(&s.env);
    s.names.set_admin(&new_admin);
    s.names.set_treasury(&new_treasury);
    let settings = s.names.settings();
    assert_eq!(settings.admin, new_admin);
    assert_eq!(settings.treasury, new_treasury);
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);
    assert_eq!(s.token_client.balance(&new_treasury), PRICE);
}

#[test]
fn price_changes_emit_an_event() {
    use soroban_sdk::testutils::Events as _;
    let s = setup();
    s.names.set_price(&(PRICE * 2));
    assert_eq!(s.env.events().all().events().len(), 1);
}

#[test]
fn short_names_can_cost_more() {
    let s = setup();
    s.names.set_length_pricing(&10, &3);
    assert_eq!(s.names.price_for(&n(&s, "abc"), &2), PRICE * 20);
    assert_eq!(s.names.price_for(&n(&s, "abcd"), &1), PRICE * 3);
    assert_eq!(s.names.price_for(&n(&s, "abcde"), &1), PRICE);
    s.names.register(&s.alice, &n(&s, "abc"), &s.alice, &1);
    assert_eq!(s.token_client.balance(&s.treasury), PRICE * 10);
    assert_eq!(
        s.names.try_set_length_pricing(&0, &1),
        Err(Ok(Error::InvalidPrice))
    );
}

#[test]
fn primary_name_can_be_cleared() {
    let s = setup();
    s.names.register(&s.alice, &n(&s, "alice"), &s.alice, &1);
    s.names.set_primary(&s.alice, &n(&s, "alice"));
    s.names.clear_primary(&s.alice);
    assert_eq!(s.names.primary_name(&s.alice), None);
}
