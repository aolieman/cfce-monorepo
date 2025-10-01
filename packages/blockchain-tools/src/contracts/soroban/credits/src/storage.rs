#![allow(non_snake_case)]

use soroban_sdk::{contracttype, Address, Bytes, Env, String};
use crate::sink_contract;

//pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
//pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
//pub(crate) const BALANCE_LIFETIME_THRESHOLD: u32 = BALANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;
//pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
//pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;


#[derive(Clone)]
#[contracttype]
pub enum DataKey {
  Admin,
  Balance,
  Initiative,
  MinimumDonation,
  Provider,
  ProviderFees,
  Vendor,
  VendorFees,
  XLMContract,
  USDCContract,
  CarbonSac,
  SinkContract,
  SoroswapRouter
}

//pub fn instance_bump(e: &Env){
//  e.storage().instance().bump(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
//}

fn zero_address(e: &Env) -> Address {
  Address::from_string_bytes(&Bytes::from_slice(&e, &[0; 32]))
  //Address::from_contract_id(&BytesN::from_array(e, &[0u8; 32]))
  //Address::new([0u8; 32])
}


//---- READ/WRITE

pub fn read_balance(e: &Env) -> i128 {
  let key = DataKey::Balance;
  match e.storage().persistent().get(&key) {
    Some(balance) => balance,
    None => 0
  }
}
/*
pub fn increment_balance(e: &Env, amount: i128) -> i128 {
  let key = DataKey::Balance;
  let val = read_balance(&e) + amount;
  e.storage().persistent().set(&key, &val);
  val
}
*/
pub fn write_balance(e: &Env, amount: i128) {
  let key = DataKey::Balance;
  e.storage().persistent().set(&key, &amount);
}

pub fn read_initiative(e: &Env) -> String {
  let key = DataKey::Initiative;
  let val = e.storage().instance().get(&key);
  match val {
    Some(amount) => amount,
    None => String::from_str(&e, "")
  }
}

pub fn write_initiative(e: &Env, value: String) {
  let key = DataKey::Initiative;
  e.storage().instance().set(&key, &value);
}

pub fn read_minimum_donation(e: &Env) -> i128 {
  let key = DataKey::MinimumDonation;
  let val = e.storage().instance().get(&key);
  match val {
    Some(amount) => amount,
    None => 0
  }
}

pub fn write_minimum_donation(e: &Env, value: i128) {
  let key = DataKey::MinimumDonation;
  e.storage().instance().set(&key, &value);
}

pub fn read_provider(e: &Env) -> Address {
  let key = DataKey::Provider;
  let val = e.storage().instance().get(&key);
  match val {
    Some(addr) => addr,
    None => zero_address(&e)
  }
}

pub fn write_provider(e: &Env, value: &Address) {
  let key = DataKey::Provider;
  e.storage().instance().set(&key, &value);
}

pub fn read_provider_fees(e: &Env) -> i128 {
  let key = DataKey::ProviderFees;
  let val = e.storage().instance().get(&key);
  match val {
    Some(amount) => amount,
    None => 0
  }
}

pub fn write_provider_fees(e: &Env, value: i128) {
  let key = DataKey::ProviderFees;
  e.storage().instance().set(&key, &value);
}

pub fn read_vendor(e: &Env) -> Address {
  let key = DataKey::Vendor;
  let val = e.storage().instance().get(&key);
  match val {
    Some(addr) => addr,
    None => zero_address(&e)
  }
}

pub fn write_vendor(e: &Env, value: &Address) {
  let key = DataKey::Vendor;
  e.storage().instance().set(&key, &value);
}

pub fn read_vendor_fees(e: &Env) -> i128 {
  let key = DataKey::VendorFees;
  let val = e.storage().instance().get(&key);
  match val {
    Some(amount) => amount,
    None => 0
  }
}

pub fn write_vendor_fees(e: &Env, value: i128) {
  let key = DataKey::VendorFees;
  e.storage().instance().set(&key, &value);
}

fn _get_contract_address(e: &Env, key: &DataKey) -> Address {
  e.storage()
    .persistent()
    .get::<_, Address>(key)
    .unwrap_or_else(|| zero_address(e))
}

pub fn read_token_contracts(e: &Env) -> (Address, Address, Address) {
  (
    _get_contract_address(e, &DataKey::XLMContract),
    _get_contract_address(e, &DataKey::USDCContract),
    _get_contract_address(e, &DataKey::CarbonSac),
  )
}

pub fn write_token_contracts(
  e: &Env,
  xlm: &Address,
  usdc: &Address,
  carbonSac: &Address
) {
  e.storage().persistent().set(&DataKey::XLMContract, &xlm);
  e.storage().persistent().set(&DataKey::USDCContract, &usdc);
  e.storage().persistent().set(&DataKey::CarbonSac, &carbonSac);
}

pub fn read_external_contracts(e: &Env) -> (Address, Address) {
  (
    _get_contract_address(e, &DataKey::SinkContract),
    _get_contract_address(e, &DataKey::SoroswapRouter)
  )
}

pub fn write_external_contracts(
  e: &Env,
  sink: &Address,
  soroswapRouter: &Address
) {
  e.storage().persistent().set(&DataKey::SinkContract, &sink);
  e.storage().persistent().set(&DataKey::SoroswapRouter, &soroswapRouter);
}

pub fn read_bucket_from_sink(e: &Env) -> i128 {
  let (sinkContractAddr, _) = read_external_contracts(e);
  let sink_client = sink_contract::Client::new(e, &sinkContractAddr);
  sink_client.get_minimum_sink_amount().into()
}