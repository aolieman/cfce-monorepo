extern crate std;
use std::rc::Rc;
use stellar_strkey;
use std::println as info;
use soroban_env_host::{budget::AsBudget, Env as _, EnvBase};
use soroban_sdk::{
  testutils::{Address as _, Ledger},
  xdr::{self, Asset, Limits, WriteXdr},
  Address, Env, FromVal, String,
  token,
};
use crate::{
  contract::Credits,
  CreditsClient,
  mintable_token,
  sink_contract,
  soroswap_router,
  soroswap_factory,
  soroswap_pair
};

pub fn deploy_native_sac(env: &Env) -> Address {
  let xdr_bytes = Asset::Native.to_xdr(Limits::none()).unwrap();
  let asset_slice = xdr_bytes.as_slice();
  let host = env.host();
  let bytes_obj = host.bytes_new_from_slice(asset_slice).unwrap();
  let sac_res = host.create_asset_contract(bytes_obj);
  Address::from_val(env, &sac_res.unwrap())
}

pub fn create_account_entry(env: &Env, pubkey: &str, balance: i64) {
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
        balance: balance, // 1000 XLM in stroops
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

pub fn setup_tokens<'a>(
  e: &Env,
  admin: &Address
) -> (
  token::Client<'a>,
  mintable_token::Client<'a>,
  mintable_token::Client<'a>,

) {
  let usdc  = mintable_token::Client::new(
    &e,
    &e.register_stellar_asset_contract_v2(admin.clone()).address()
  );

  let xlm_address = deploy_native_sac(&e);
  let xlm = token::Client::new(&e, &xlm_address);

  let carbonSac = mintable_token::Client::new(
    &e,
    &e.register_stellar_asset_contract_v2(admin.clone()).address()
  );

  (xlm, usdc, carbonSac)
}

pub fn create_credit_contract<'a>(
  e: &Env,
  admin: &Address,
  initiative: &String,
  provider: &Address,
  vendor: &Address,
  bucket: i128,
  xlm: &Address,
  usdc: &Address,
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
      usdc.clone(),
      carbonSac.clone(),
      sink.clone(),
      soroswapRouter.clone()
    )
  );
  let contract_client = CreditsClient::new(e, &contract_id);
  info!("Credit Contract created!");
  contract_client
}

pub fn create_sink_successors(e: &Env) -> (Address, Address) {
  let admin = Address::generate(e);
  let carbon_sac = Address::generate(e);
  let carbonsink_sac = Address::generate(e);

  let first_sink_id = e.register(
    sink_contract::WASM, 
    (&admin, &carbon_sac, &carbonsink_sac)
  );
  let first_sink_client = sink_contract::Client::new(e, &first_sink_id);
  let second_sink_id = e.register(
    sink_contract::WASM, 
    (&admin, &carbon_sac, &carbonsink_sac)
  );
  first_sink_client.set_contract_successor(&second_sink_id);

  let second_sink_client = sink_contract::Client::new(e, &first_sink_id);
  let third_sink_id = e.register(
    sink_contract::WASM, 
    (&admin, &carbon_sac, &carbonsink_sac)
  );
  second_sink_client.set_contract_successor(&third_sink_id);

  (first_sink_id, third_sink_id)
}

pub fn setup_soroswap_env<'a>(
  e: &Env,
  admin: &Address,
  xlm: &token::Client<'a>,
  usdc: &mintable_token::Client<'a>,
  carbonSac: &mintable_token::Client<'a>,
) -> soroswap_router::Client<'a> {
  let xlm_liqudity: i64 = 1_000_000_000_000_000;
  let usdc_liquidity: i128 = 1_000_000_000_000_000;
  let carbon_liquidity: i128 = 1_000_000_000_000_000;

  let xlm_minter_pubkey = "GA2H3SJYGIUG2DXXUZ7IN3LNO2AIMVWCDCL25PKQHKMC76OWW3HYQHY4";
  let xlm_minter_address = Address::from_str(&e, xlm_minter_pubkey);
  create_account_entry(&e, &xlm_minter_pubkey, i64::MAX);

  xlm.transfer(&xlm_minter_address, &admin, &(i64::MAX as i128));
  usdc.mint(&admin, &(usdc_liquidity * 2));
  carbonSac.mint(&admin, &(carbon_liquidity * 2));

  // prepare soroswap factory
  let pair_hash = e.deployer().upload_contract_wasm(soroswap_pair::WASM);
  let factory_address = e.register(
    soroswap_factory::WASM,
    ()
  );
  let factory_client = soroswap_factory::Client::new(&e, &factory_address);
  factory_client.initialize(&admin, &pair_hash);

  // prepare two pairs
  factory_client.create_pair(&xlm.address, &usdc.address);
  factory_client.create_pair(&usdc.address, &carbonSac.address);

  // prepare router
  let router_address = e.register(
    soroswap_router::WASM,
    ()
  );
  let router_client = soroswap_router::Client::new(&e, &router_address);
  router_client.initialize(&factory_address);

  // add liquidity for usdc <-> carbon
  let ledger_timestamp = 100;
  let first_liquidity_add_desired_deadline = 1000;

  e.ledger().with_mut(|li| {
    li.timestamp = ledger_timestamp;
  });

  router_client.add_liquidity(
    &usdc.address,
    &carbonSac.address,
    &usdc_liquidity,
    &carbon_liquidity,
    &0,
    &0,
    &admin,
    &first_liquidity_add_desired_deadline
  );

  // add liqudiity for xlm <-> usdc
  e.ledger().with_mut(|li| {
    li.timestamp = first_liquidity_add_desired_deadline + 1;
  });
  let second_liquidity_add_desired_deadline = 2000;

  router_client.add_liquidity(
    &xlm.address,
    &usdc.address,
    &(xlm_liqudity as i128),
    &usdc_liquidity,
    &0,
    &0,
    &admin,
    &second_liquidity_add_desired_deadline
  );

  router_client
}

pub struct CreditTest<'a> {
  pub e: Env,
  pub soroswap_router: soroswap_router::Client<'a>,
  pub xlm: token::Client<'a>,
  pub usdc: mintable_token::Client<'a>,
  pub carbonSac: mintable_token::Client<'a>,
  pub admin: Address,
  pub initiative: String,
  pub bucket: i128,
  pub provider: Address,
  pub vendor: Address,
  pub credit: CreditsClient<'a>
}

impl<'a> CreditTest<'a> {
  pub fn setup() -> Self {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let (xlm, usdc, carbonSac) = setup_tokens(&e, &admin);

    let soroswap_router = setup_soroswap_env(
      &e,
      &admin,
      &xlm,
      &usdc,
      &carbonSac
    );

    let initiative = String::from_str(&e, "30c0636f-b0f1-40d5-bb9c-a531dc4d69e2");
    let bucket = 200_000_000;
    let provider = Address::generate(&e);
    let vendor = Address::generate(&e);

    let (first_sink, _) = create_sink_successors(&e);

    let credit = create_credit_contract(
      &e,
      &admin,
      &initiative,
      &provider,
      &vendor,
      bucket,
      &xlm.address,
      &usdc.address,
      &carbonSac.address,
      &first_sink,
      &soroswap_router.address
    );

    CreditTest {
      e,
      soroswap_router,
      xlm,
      usdc,
      carbonSac,
      admin,
      initiative,
      bucket,
      provider,
      vendor,
      credit
    }
  }
}
