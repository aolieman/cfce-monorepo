#![allow(non_snake_case)]
use crate::admin::{check_admin, read_administrator, write_administrator};
use crate::events;
use crate::storage::{
  read_balance, write_balance,
  read_initiative, write_initiative,
  read_minimum_donation, write_minimum_donation,
  read_provider, write_provider,
  read_provider_fees, write_provider_fees,
  read_vendor, write_vendor,
  read_vendor_fees, write_vendor_fees,
  read_token_contracts, write_token_contracts,
  read_external_contracts, write_external_contracts,
  read_bucket_from_sink,
};

use soroban_sdk::{
  contract, contractimpl, token,
  Address, Env, String, Error, Vec
};
use crate::sink_contract;
use crate::soroswap_router;

#[contract]
pub struct Credits;

#[contractimpl]
impl Credits {
  pub fn __constructor(
    e: Env,
    admin: Address,
    initiative: String,
    provider: Address,
    vendor: Address,
    xlm: Address,
    usdc: Address,
    carbonSac: Address,
    sink: Address,
    soroswapRouter: Address
  ) -> Result<(), Error> {
    write_administrator(&e, &admin);
    write_balance(&e, 0);
    write_initiative(&e, initiative);
    write_minimum_donation(&e, 1000000);
    write_provider(&e, &provider);
    write_provider_fees(&e, 90);
    write_vendor(&e, &vendor);
    write_vendor_fees(&e, 10);
    write_token_contracts(&e, &xlm, &usdc, &carbonSac);
    write_external_contracts(&e, &sink, &soroswapRouter);
    Ok(())
  }

  //---- METHODS

  pub fn register(_: Env, from: Address) {
    from.require_auth();
  }

  pub fn donate(e: Env, from: Address, amount: i128) {
    if amount <= 0 { panic!("amount less than zero") }
    let minimum = read_minimum_donation(&e);
    if amount < minimum { panic!("amount less than minimum allowed") }
    from.require_auth();
    let thisctr = &e.current_contract_address();
    let provider = read_provider(&e);
    let providerFees = read_provider_fees(&e);
    let vendorFees = read_vendor_fees(&e);
    let balance = read_balance(&e);
    let bucket = read_bucket_from_sink(&e);

    // instance_bump(&e);
    let (ctr, _, _) = read_token_contracts(&e);
    let xlm: token::TokenClient<'_> = token::Client::new(&e, &ctr);
    xlm.transfer(&from, &thisctr, &amount); // From donor to contract

    let xlmProviderFees = (amount * providerFees / 100) as i128;
    let carbonProviderfees = Self::swap_xlm_to_carbon(
      e.clone(),
      thisctr.clone(),
      xlmProviderFees
    );

    let xlmVendorFees = (amount * vendorFees / 100) as i128;

    if xlmVendorFees > 0 {
      let vendor = read_vendor(&e);
      xlm.transfer(&thisctr, &vendor, &xlmVendorFees); // Vendor fees from contract to vendor
    }
    let newbalance = balance + carbonProviderfees; // Accumulate carbon credits
    if newbalance >= bucket {
      let reminder = newbalance % bucket;
      let credits  = newbalance - reminder;
      xlm.transfer(&thisctr, &provider, &credits); // Credits from contract to provider
      write_balance(&e, reminder);
    } else {
      write_balance(&e, newbalance);
    }

    events::donation(&e, from, provider, amount);
  }

  pub fn swap_xlm_to_carbon(e: Env, from: Address, xlm_amount: i128) -> i128 {
    if xlm_amount <= 0 { panic!("amount less than zero") }
    from.require_auth();

    let (xlm, usdc, carbon) = read_token_contracts(&e);

    let (_, soroswapRouter) = read_external_contracts(&e);
    let soroswap_router_client = soroswap_router::Client::new(&e, &soroswapRouter);

    let mut path: Vec<Address> = Vec::new(&e);
    path.push_back(xlm.clone());
    path.push_back(usdc.clone());
    path.push_back(carbon.clone());

    let deadline = e.ledger().timestamp() + 60;  // valid for 1 min

    // TODO: consider adding a carbon_minimum argumen
    let executed_amounts = soroswap_router_client.swap_exact_tokens_for_tokens(
      &xlm_amount, // amount_in
      &0,           // amount_out_min
      &path,        // path 
      &from,        // to 
      &deadline,    // deadline
    );

    // executed_amounts.get(0): amount_in, 1: first_out, 2: expected_amount_out);
    executed_amounts.get(2).unwrap()
  }

  //---- VIEWS

  pub fn getAdmin(e: Env) -> Address {
    read_administrator(&e)
  }

  pub fn getBalance(e: Env) -> i128 {
    read_balance(&e)
  }

  pub fn getContractXLMBalance(e: Env) -> i128 {
    let adr = e.current_contract_address();
    let (ctr, _, _) = read_token_contracts(&e);
    let xlm = token::Client::new(&e, &ctr);
    xlm.balance(&adr)
  }

  pub fn getBucket(e: Env) -> i128 {
    read_bucket_from_sink(&e)
  }

  pub fn getInitiative(e: Env) -> String {
    read_initiative(&e)
  }

  pub fn getMinimum(e: Env) -> i128 {
    read_minimum_donation(&e)
  }

  pub fn getProvider(e: Env) -> Address {
    read_provider(&e)
  }

  pub fn getProviderFees(e: Env) -> i128 {
    read_provider_fees(&e)
  }

  pub fn getVendor(e: Env) -> Address {
    read_vendor(&e)
  }

  pub fn getVendorFees(e: Env) -> i128 {
    read_vendor_fees(&e)
  }

  pub fn getTokens(e: Env) -> (Address, Address, Address) {
    // read_xlm_contract(&e)
    read_token_contracts(&e)
  }

  pub fn getExternalContracts(e: Env) -> (Address, Address) {
    read_external_contracts(&e)
  }

  //---- UPDATES

  pub fn setAdmin(e: Env, newval: Address) {
    check_admin(&e);
    //instance_bump(&e);
    let admin = read_administrator(&e);
    write_administrator(&e, &newval);
    events::admin(&e, admin, newval);
  }

  pub fn setMinimum(e: Env, newval: i128) {
    check_admin(&e);
    //instance_bump(&e);
    let oldval = read_minimum_donation(&e);
    write_minimum_donation(&e, newval);
    events::minimum(&e, oldval, newval);
  }

  pub fn setProvider(e: Env, newval: Address) {
    check_admin(&e);
    //instance_bump(&e);
    let oldval = read_provider(&e);
    write_provider(&e, &newval);
    events::provider(&e, oldval, newval);
  }

  pub fn setProviderFees(e: Env, newval: i128) {
    check_admin(&e);
    //instance_bump(&e);
    let oldval = read_provider_fees(&e);
    write_provider_fees(&e, newval);
    events::providerFees(&e, oldval, newval);
  }

  pub fn setVendor(e: Env, newval: Address) {
    check_admin(&e);
    //instance_bump(&e);
    let oldval = read_vendor(&e);
    write_vendor(&e, &newval);
    events::vendor(&e, oldval, newval);
  }

  pub fn setVendorFees(e: Env, newval: i128) {
    check_admin(&e);
    //instance_bump(&e);
    let oldval = read_vendor_fees(&e);
    write_vendor_fees(&e, newval);
    events::vendorFees(&e, oldval, newval);
  }

  pub fn setXLMToken(e: Env, newval: Address) {
    check_admin(&e);
    //instance_bump(&e);
    let (oldval, usdc, carbonSac) = read_token_contracts(&e);
    write_token_contracts(&e, &newval, &usdc, &carbonSac);
    events::xlmChange(&e, oldval, newval);
  }

  pub fn setUSDCToken(e: Env, newval: Address) {
    check_admin(&e);
    //instance_bump(&e);
    let (xlm, oldval, carbonSac) = read_token_contracts(&e);
    write_token_contracts(&e, &xlm, &newval, &carbonSac);
    events::usdcChange(&e, oldval, newval);
  }

  pub fn setCarbonSacToken(e: Env, newval: Address) {
    check_admin(&e);
    //instance_bump(&e);
    let (xlm, usdc, oldval) = read_token_contracts(&e);
    write_token_contracts(&e, &xlm, &usdc, &newval);
    events::usdcChange(&e, oldval, newval);
  }

  pub fn setSinkToSuccessor(e: Env) {
    check_admin(&e);
    //instance_bump(&e);
    let (oldAddr, soroswapRouter) = read_external_contracts(&e);
    let mut sinkContractAddr = oldAddr.clone();

    loop {
      let sink_client = sink_contract::Client::new(&e, &sinkContractAddr);
      let successor = sink_client.get_contract_successor();

      if sinkContractAddr == successor {
        break;
      }
      sinkContractAddr = successor;
    }

    write_external_contracts(&e, &sinkContractAddr, &soroswapRouter);
    events::sinkChange(&e, oldAddr, sinkContractAddr);
  }

  pub fn setSoroswapRouter(e: Env, newval: Address) {
    check_admin(&e);
    //instance_bump(&e);
    let (sink, oldval) = read_external_contracts(&e);
    write_external_contracts(&e, &sink, &newval);
    events::soroswapRouterChange(&e, oldval, newval);
  }
}
