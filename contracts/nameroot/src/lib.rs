#![no_std]

//! Nameroot: human-readable names for Stellar addresses.
//!
//! Register `alice` once and people can pay `alice` instead of copying a
//! 56-character key. Each name has an **owner** (who controls it) and a
//! **target** (the address it resolves to; they can differ, e.g. a name
//! owned by a cold wallet that points at a hot wallet).
//!
//! Names are leased by the year, paid in a configurable token to a
//! treasury. That keeps squatting costly and lets abandoned names return
//! to the pool. After expiry there's a 30-day grace period in which only
//! the old owner can renew; then anyone can register it. Expired names
//! never resolve, so a lapsed name can't silently redirect payments.
//!
//! Reverse records ("which name is this address?") are only returned while
//! the name still points at that address, so a stale or spoofed reverse
//! lookup is impossible.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env, String,
};

pub const MIN_NAME_LEN: u32 = 3;
pub const MAX_NAME_LEN: u32 = 32;
pub const YEAR: u64 = 365 * 24 * 60 * 60;
pub const GRACE_PERIOD: u64 = 30 * 24 * 60 * 60;
pub const MAX_YEARS: u32 = 10;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub owner: Address,
    pub target: Address,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settings {
    pub admin: Address,
    pub fee_token: Address,
    pub price_per_year: i128,
    pub treasury: Address,
}

/// Price multipliers for short names (applied to price_per_year).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LengthPricing {
    /// Multiplier for 3-character names.
    pub three: u32,
    /// Multiplier for 4-character names.
    pub four: u32,
}

#[contracttype]
pub enum DataKey {
    Settings,
    Name(String),
    Primary(Address),
    LengthPricing,
    /// `label.parent` → who set it and where it points.
    Sub(String, String),
}

/// A subname such as `pay.alice`. It only resolves while `owner` still owns
/// the parent name, so a re-registered or transferred parent never inherits
/// someone else's subnames.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubRecord {
    pub owner: Address,
    pub target: Address,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Kept for stable error codes; the registry is configured by its constructor.
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidName = 3,
    NameTaken = 4,
    NameNotFound = 5,
    NameExpired = 6,
    InvalidYears = 7,
    NotOwner = 8,
    InvalidPrice = 9,
    PrimaryMismatch = 10,
    SubnameNotFound = 11,
}

#[contractevent(topics = ["name", "registered"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Registered {
    #[topic]
    pub name: String,
    pub owner: Address,
    pub expires_at: u64,
}

#[contractevent(topics = ["name", "price"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceChanged {
    pub old: i128,
    pub new: i128,
}

#[contractevent(topics = ["name", "admin"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminChanged {
    pub from: Address,
    pub to: Address,
}

#[contractevent(topics = ["name", "treasury"], data_format = "single-value")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryChanged {
    pub treasury: Address,
}

#[contractevent(topics = ["name", "unprimary"], data_format = "single-value")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimaryCleared {
    pub address: Address,
}

#[contractevent(topics = ["name", "subname"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnameSet {
    #[topic]
    pub name: String,
    pub label: String,
    pub target: Address,
}

#[contractevent(topics = ["name", "renewed"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Renewed {
    #[topic]
    pub name: String,
    pub expires_at: u64,
}

#[contractevent(topics = ["name", "updated"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Updated {
    #[topic]
    pub name: String,
    pub owner: Address,
    pub target: Address,
}

const DAY_IN_LEDGERS: u32 = 17_280;
const BUMP_THRESHOLD: u32 = 30 * DAY_IN_LEDGERS;
const BUMP_TO: u32 = 365 * DAY_IN_LEDGERS;

#[contract]
pub struct Nameroot;

#[contractimpl]
impl Nameroot {
    /// Configure the registry at deployment. Being a constructor, it runs in
    /// the same transaction as the deploy, so nobody else can claim the
    /// admin and treasury of a freshly deployed registry.
    pub fn __constructor(
        env: Env,
        admin: Address,
        fee_token: Address,
        price_per_year: i128,
        treasury: Address,
    ) -> Result<(), Error> {
        if price_per_year < 0 {
            return Err(Error::InvalidPrice);
        }
        let settings = Settings {
            admin,
            fee_token,
            price_per_year,
            treasury,
        };
        env.storage().instance().set(&DataKey::Settings, &settings);
        env.storage().instance().extend_ttl(BUMP_THRESHOLD, BUMP_TO);
        Ok(())
    }

    /// Register an available name for `years` years, paying the fee.
    pub fn register(
        env: Env,
        owner: Address,
        name: String,
        target: Address,
        years: u32,
    ) -> Result<u64, Error> {
        owner.require_auth();
        validate_name(&name)?;
        check_years(years)?;
        let now = env.ledger().timestamp();
        if let Some(existing) = get_record(&env, &name) {
            // Taken while active, and reserved for the old owner during grace.
            if now < existing.expires_at + GRACE_PERIOD {
                return Err(Error::NameTaken);
            }
        }
        charge(&env, &owner, &name, years)?;

        let expires_at = now + YEAR * years as u64;
        let record = Record {
            owner: owner.clone(),
            target,
            expires_at,
        };
        save(&env, &name, &record);
        Registered {
            name,
            owner,
            expires_at,
        }
        .publish(&env);
        Ok(expires_at)
    }

    /// Extend a name. Anyone may pay, but during the grace period only for
    /// the existing owner's benefit (ownership never changes here).
    pub fn renew(env: Env, payer: Address, name: String, years: u32) -> Result<u64, Error> {
        payer.require_auth();
        check_years(years)?;
        let mut record = get_record(&env, &name).ok_or(Error::NameNotFound)?;
        let now = env.ledger().timestamp();
        if now >= record.expires_at + GRACE_PERIOD {
            return Err(Error::NameExpired);
        }
        charge(&env, &payer, &name, years)?;
        let base = if record.expires_at > now {
            record.expires_at
        } else {
            now
        };
        let max = now + YEAR * MAX_YEARS as u64;
        record.expires_at = (base + YEAR * years as u64).min(max);
        save(&env, &name, &record);
        Renewed {
            name,
            expires_at: record.expires_at,
        }
        .publish(&env);
        Ok(record.expires_at)
    }

    /// Point a name at a different address. Owner only, while active.
    pub fn set_target(env: Env, name: String, target: Address) -> Result<(), Error> {
        let mut record = active_record(&env, &name)?;
        record.owner.require_auth();
        record.target = target;
        save(&env, &name, &record);
        Updated {
            name,
            owner: record.owner,
            target: record.target,
        }
        .publish(&env);
        Ok(())
    }

    /// Hand a name to a new owner. Owner only, while active.
    pub fn transfer(env: Env, name: String, new_owner: Address) -> Result<(), Error> {
        let mut record = active_record(&env, &name)?;
        record.owner.require_auth();
        // A transferred name points at its new owner; leaving the old target
        // would keep paying the previous owner until they noticed.
        record.owner = new_owner.clone();
        record.target = new_owner;
        save(&env, &name, &record);
        Updated {
            name,
            owner: record.owner,
            target: record.target,
        }
        .publish(&env);
        Ok(())
    }

    /// Address a name currently resolves to. Fails for expired names.
    /// Point `label.name` at `target`. Owner of an active `name` only.
    pub fn set_subname(
        env: Env,
        name: String,
        label: String,
        target: Address,
    ) -> Result<(), Error> {
        let record = active_record(&env, &name)?;
        record.owner.require_auth();
        validate_name(&label)?;
        let key = DataKey::Sub(name.clone(), label.clone());
        env.storage().persistent().set(
            &key,
            &SubRecord {
                owner: record.owner,
                target: target.clone(),
            },
        );
        env.storage()
            .persistent()
            .extend_ttl(&key, BUMP_THRESHOLD, BUMP_TO);
        SubnameSet {
            name,
            label,
            target,
        }
        .publish(&env);
        Ok(())
    }

    /// Remove `label.name`. Owner of `name` only.
    pub fn remove_subname(env: Env, name: String, label: String) -> Result<(), Error> {
        let record = active_record(&env, &name)?;
        record.owner.require_auth();
        let key = DataKey::Sub(name, label);
        if !env.storage().persistent().has(&key) {
            return Err(Error::SubnameNotFound);
        }
        env.storage().persistent().remove(&key);
        Ok(())
    }

    /// Target of `label.name`, if it exists and the parent's owner set it.
    pub fn subname(env: Env, name: String, label: String) -> Option<Address> {
        let parent = active_record(&env, &name).ok()?;
        let sub: SubRecord = env.storage().persistent().get(&DataKey::Sub(name, label))?;
        (sub.owner == parent.owner).then_some(sub.target)
    }

    /// Resolve `alice` or a subname like `pay.alice`.
    pub fn resolve(env: Env, name: String) -> Result<Address, Error> {
        if let Some((label, parent)) = split_subname(&env, &name) {
            return Self::subname(env, parent, label).ok_or(Error::SubnameNotFound);
        }
        Ok(active_record(&env, &name)?.target)
    }

    pub fn get_record(env: Env, name: String) -> Result<Record, Error> {
        get_record(&env, &name).ok_or(Error::NameNotFound)
    }

    pub fn is_available(env: Env, name: String) -> Result<bool, Error> {
        validate_name(&name)?;
        Ok(match get_record(&env, &name) {
            None => true,
            Some(r) => env.ledger().timestamp() >= r.expires_at + GRACE_PERIOD,
        })
    }

    /// Choose which name `address` shows as. The name must point at it.
    pub fn set_primary(env: Env, address: Address, name: String) -> Result<(), Error> {
        address.require_auth();
        let record = active_record(&env, &name)?;
        if record.target != address {
            return Err(Error::PrimaryMismatch);
        }
        let key = DataKey::Primary(address);
        env.storage().persistent().set(&key, &name);
        env.storage()
            .persistent()
            .extend_ttl(&key, BUMP_THRESHOLD, BUMP_TO);
        Ok(())
    }

    /// Reverse lookup. Only answers while the primary name is active and
    /// still points at `address`.
    pub fn primary_name(env: Env, address: Address) -> Option<String> {
        let name: String = env
            .storage()
            .persistent()
            .get(&DataKey::Primary(address.clone()))?;
        let record = active_record(&env, &name).ok()?;
        (record.target == address).then_some(name)
    }

    pub fn set_price(env: Env, price_per_year: i128) -> Result<(), Error> {
        let mut settings = settings(&env)?;
        settings.admin.require_auth();
        if price_per_year < 0 {
            return Err(Error::InvalidPrice);
        }
        let old = settings.price_per_year;
        settings.price_per_year = price_per_year;
        env.storage().instance().set(&DataKey::Settings, &settings);
        PriceChanged {
            old,
            new: price_per_year,
        }
        .publish(&env);
        Ok(())
    }

    pub fn settings(env: Env) -> Result<Settings, Error> {
        settings(&env)
    }

    /// Hand the registry to a new admin. Both sign, so a typo can't lock it.
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        let mut s = settings(&env)?;
        s.admin.require_auth();
        new_admin.require_auth();
        let from = s.admin.clone();
        s.admin = new_admin.clone();
        env.storage().instance().set(&DataKey::Settings, &s);
        AdminChanged {
            from,
            to: new_admin,
        }
        .publish(&env);
        Ok(())
    }

    /// Send future registration fees somewhere else. Admin only.
    pub fn set_treasury(env: Env, treasury: Address) -> Result<(), Error> {
        let mut s = settings(&env)?;
        s.admin.require_auth();
        s.treasury = treasury.clone();
        env.storage().instance().set(&DataKey::Settings, &s);
        TreasuryChanged { treasury }.publish(&env);
        Ok(())
    }

    /// Multipliers for 3- and 4-character names (1 = normal price). Admin only.
    pub fn set_length_pricing(env: Env, three: u32, four: u32) -> Result<(), Error> {
        let s = settings(&env)?;
        s.admin.require_auth();
        if three == 0 || four == 0 {
            return Err(Error::InvalidPrice);
        }
        env.storage()
            .instance()
            .set(&DataKey::LengthPricing, &LengthPricing { three, four });
        Ok(())
    }

    pub fn length_pricing(env: Env) -> LengthPricing {
        length_pricing(&env)
    }

    /// What registering or renewing `name` for `years` costs, in the fee token.
    pub fn price_for(env: Env, name: String, years: u32) -> Result<i128, Error> {
        validate_name(&name)?;
        check_years(years)?;
        fee(&env, &name, years)
    }

    /// Remove the caller's reverse record.
    pub fn clear_primary(env: Env, address: Address) -> Result<(), Error> {
        address.require_auth();
        env.storage()
            .persistent()
            .remove(&DataKey::Primary(address.clone()));
        PrimaryCleared { address }.publish(&env);
        Ok(())
    }
}

/// 3–32 chars of a–z, 0–9 and '-', not starting or ending with '-'.
/// Lowercase only, so `Alice` and `alice` can't be two different names.
fn validate_name(name: &String) -> Result<(), Error> {
    let len = name.len();
    if !(MIN_NAME_LEN..=MAX_NAME_LEN).contains(&len) {
        return Err(Error::InvalidName);
    }
    let mut buf = [0u8; MAX_NAME_LEN as usize];
    let bytes = &mut buf[..len as usize];
    name.copy_into_slice(bytes);
    for (i, &b) in bytes.iter().enumerate() {
        let ok = b.is_ascii_lowercase()
            || b.is_ascii_digit()
            || (b == b'-' && i != 0 && i + 1 != bytes.len());
        if !ok {
            return Err(Error::InvalidName);
        }
    }
    Ok(())
}

/// "pay.alice" → Some(("pay", "alice")); names without a dot → None.
fn split_subname(env: &Env, name: &String) -> Option<(String, String)> {
    let len = name.len() as usize;
    if len > 2 * MAX_NAME_LEN as usize + 1 {
        return None;
    }
    let mut buf = [0u8; 2 * MAX_NAME_LEN as usize + 1];
    let bytes = &mut buf[..len];
    name.copy_into_slice(bytes);
    let dot = bytes.iter().position(|&b| b == b'.')?;
    Some((
        String::from_bytes(env, &bytes[..dot]),
        String::from_bytes(env, &bytes[dot + 1..]),
    ))
}

fn check_years(years: u32) -> Result<(), Error> {
    if years == 0 || years > MAX_YEARS {
        Err(Error::InvalidYears)
    } else {
        Ok(())
    }
}

fn length_pricing(env: &Env) -> LengthPricing {
    env.storage()
        .instance()
        .get(&DataKey::LengthPricing)
        .unwrap_or(LengthPricing { three: 1, four: 1 })
}

fn fee(env: &Env, name: &String, years: u32) -> Result<i128, Error> {
    let s = settings(env)?;
    let lp = length_pricing(env);
    let multiplier = match name.len() {
        3 => lp.three,
        4 => lp.four,
        _ => 1,
    } as i128;
    s.price_per_year
        .checked_mul(years as i128)
        .and_then(|v| v.checked_mul(multiplier))
        .ok_or(Error::InvalidPrice)
}

fn charge(env: &Env, payer: &Address, name: &String, years: u32) -> Result<(), Error> {
    let s = settings(env)?;
    let fee = fee(env, name, years)?;
    if fee > 0 {
        token::Client::new(env, &s.fee_token).transfer(payer, &s.treasury, &fee);
    }
    Ok(())
}

fn settings(env: &Env) -> Result<Settings, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Settings)
        .ok_or(Error::NotInitialized)
}

fn get_record(env: &Env, name: &String) -> Option<Record> {
    env.storage().persistent().get(&DataKey::Name(name.clone()))
}

fn active_record(env: &Env, name: &String) -> Result<Record, Error> {
    let record = get_record(env, name).ok_or(Error::NameNotFound)?;
    if env.ledger().timestamp() >= record.expires_at {
        return Err(Error::NameExpired);
    }
    Ok(record)
}

fn save(env: &Env, name: &String, record: &Record) {
    let key = DataKey::Name(name.clone());
    env.storage().persistent().set(&key, record);
    env.storage()
        .persistent()
        .extend_ttl(&key, BUMP_THRESHOLD, BUMP_TO);
    env.storage().instance().extend_ttl(BUMP_THRESHOLD, BUMP_TO);
}

mod test;
