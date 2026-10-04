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

#[contracttype]
pub enum DataKey {
    Settings,
    Name(String),
    Primary(Address),
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
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
}

#[contractevent(topics = ["name", "registered"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Registered {
    #[topic]
    pub name: String,
    pub owner: Address,
    pub expires_at: u64,
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
    pub fn init(
        env: Env,
        admin: Address,
        fee_token: Address,
        price_per_year: i128,
        treasury: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Settings) {
            return Err(Error::AlreadyInitialized);
        }
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
        charge(&env, &owner, years)?;

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
        charge(&env, &payer, years)?;
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
        record.owner = new_owner;
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
    pub fn resolve(env: Env, name: String) -> Result<Address, Error> {
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
        settings.price_per_year = price_per_year;
        env.storage().instance().set(&DataKey::Settings, &settings);
        Ok(())
    }

    pub fn settings(env: Env) -> Result<Settings, Error> {
        settings(&env)
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

fn check_years(years: u32) -> Result<(), Error> {
    if years == 0 || years > MAX_YEARS {
        Err(Error::InvalidYears)
    } else {
        Ok(())
    }
}

fn charge(env: &Env, payer: &Address, years: u32) -> Result<(), Error> {
    let s = settings(env)?;
    let fee = s.price_per_year * years as i128;
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
