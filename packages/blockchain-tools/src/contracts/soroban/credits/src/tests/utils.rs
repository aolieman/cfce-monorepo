#![cfg(test)]

extern crate std;
use std::rc::Rc;

use soroban_env_host::{budget::AsBudget, Env as _, EnvBase};
use soroban_sdk::{
  xdr,
  xdr::{Asset, Limits, WriteXdr},
  Address, Env, FromVal, IntoVal, String,
  token::StellarAssetClient,
  testutils::{Address as _, IssuerFlags, MockAuth, MockAuthInvoke},
};
use stellar_strkey;
use crate::sink_contract;
use std::{println as info, println as warn};
use crate::{contract::Credits, CreditsClient};

pub fn deploy_native_sac(env: &Env) -> Address {
  let xdr_bytes = Asset::Native.to_xdr(Limits::none()).unwrap();
  let asset_slice = xdr_bytes.as_slice();
  let host = env.host();
  let bytes_obj = host.bytes_new_from_slice(asset_slice).unwrap();
  let sac_res = host.create_asset_contract(bytes_obj);
  Address::from_val(env, &sac_res.unwrap())
}

pub fn create_account_entry(env: &Env, pubkey: &str) {
  // Parse the public key from the Stellar address
  let raw_pubkey = stellar_strkey::ed25519::PublicKey::from_string(pubkey).unwrap().0;
  
  // Create an AccountId XDR
  let account_id = xdr::AccountId(
    xdr::PublicKey::PublicKeyTypeEd25519(xdr::Uint256(raw_pubkey))
  );
  
  // Create the account entry
  env.host().with_mut_storage(|storage| {
    let key = Rc::new(xdr::LedgerKey::Account(xdr::LedgerKeyAccount {
      account_id: account_id.clone(),
    }));
      
    // Create account entry data with basic values
    let entry = Rc::new(xdr::LedgerEntry {
      last_modified_ledger_seq: 0,
      data: xdr::LedgerEntryData::Account(xdr::AccountEntry {
        account_id: account_id,
        balance: 10_000_000_000, // 1000 XLM in stroops
        seq_num: xdr::SequenceNumber(1),
        num_sub_entries: 0,
        inflation_dest: None,
        flags: 0,
        home_domain: xdr::String32::default(),
        thresholds: xdr::Thresholds([1, 0, 0, 0]),
        signers: xdr::VecM::default(),
        ext: xdr::AccountEntryExt::V0,
      }),
      ext: xdr::LedgerEntryExt::V0,
    });
      
    // Add the entry to storage
    storage.put(
      &key,
      &entry,
      None,
      env.host().as_budget(),
    ).unwrap();
    
    Ok(())
  }).unwrap();
}


pub struct SinkCarbonSetup<'a> {
  pub env: Env,
  // pub funder: Address,
  // pub carbon_sac: StellarAssetContract,
  pub carbonsink_issuer: Address,
  // pub carbonsink_sac: StellarAssetContract,
  pub contract_id: Address,
  pub sink_client: sink_contract::Client<'a>,
}

  
pub fn set_up_contracts_and_funder<'a>(funder_balance: i128, env_opt: Option<Env>) -> SinkCarbonSetup<'a> {
  let env = env_opt.unwrap_or_default();

  let funder = Address::generate(&env);
  let carbon_issuer = Address::generate(&env);  // this is a C-address
  let carbonsink_issuer = Address::generate(&env);  // this is a C-address
  let carbon_sac = env.register_stellar_asset_contract_v2(carbon_issuer.clone());
  let carbonsink_sac = env.register_stellar_asset_contract_v2(carbonsink_issuer.clone());
  // WARNING: carbon_sac.issuer().address() is a G-address (some conversion by testutils)
  carbonsink_sac.issuer().set_flag(IssuerFlags::RevocableFlag);
  carbonsink_sac.issuer().set_flag(IssuerFlags::RequiredFlag);

  // set CarbonSINK issuer as the sink contract admin
  let contract_id = env.register(
      sink_contract::WASM, 
      (&carbonsink_issuer, &carbon_sac.address(), &carbonsink_sac.address())
  );
  let carbon_sac_client = StellarAssetClient::new(&env, &carbon_sac.address());
  let carbonsink_sac_client = StellarAssetClient::new(&env, &carbonsink_sac.address());

  // set the sink contract as the CarbonSINK SAC admin
  carbonsink_sac_client
    .mock_auths(&[MockAuth {
      address: &carbonsink_issuer,
      invoke: &MockAuthInvoke {
        contract: &carbonsink_sac.address(),
        fn_name: "set_admin",
        args: (&contract_id,).into_val(&env),
        sub_invokes: &[],
      },
    }])
    .set_admin(&contract_id);

  // give the funder an initial balance of `funder_balance` CARBON
  carbon_sac_client
    .mock_auths(&[MockAuth {
      address: &carbon_issuer,
      invoke: &MockAuthInvoke {
        contract: &carbon_sac.address(),
        fn_name: "mint",
        args: (&funder, &funder_balance).into_val(&env),
        sub_invokes: &[],
      },
    }])
    .mint(&funder, &funder_balance);

  let sink_client = sink_contract::Client::new(&env, &contract_id);

  SinkCarbonSetup {
    env, carbonsink_issuer, sink_client,
    contract_id,
    // carbonsink_sac, funder, carbon_sac
  }
}
  
  
pub fn create_credit_contract<'a>(
  e: &Env,
  admin: &Address,
  initiative: &String,
  provider: &Address,
  vendor: &Address,
  bucket: i128,
  xlm: &Address,
  carbonSac: &Address,
  sink: &Address,
  soroswapRouter: &Address
) -> CreditsClient<'a> {
  info!("Creating credit contract...");

  let contract_id = e.register(
    Credits,
    (
      admin.clone(),
      initiative.clone(),
      provider.clone(),
      vendor.clone(),
      bucket,
      xlm.clone(),
      carbonSac.clone(),
      sink.clone(),
      soroswapRouter.clone()
    )
  );
  let contract_client = CreditsClient::new(e, &contract_id);
  warn!("Credit Contract created!");
  contract_client
}

