extern crate std;

use soroban_sdk::{
  testutils::{
    Address as _,
    Events
  },
  Address, Env, String, IntoVal, Symbol,
  token, vec,
};

use crate::tests::utils::{
  deploy_native_sac,
  create_account_entry,
  create_credit_contract,
  create_sink_successors
};

#[test]
fn test_views() {
  let e = Env::default();
  e.mock_all_auths();

  let admin = Address::generate(&e);
  let bucket = 200_000_000_i128;
  let initiative = String::from_str(&e, "30c0636f-b0f1-40d5-bb9c-a531dc4d69e2");
  let provider = Address::generate(&e);
  let vendor = Address::generate(&e);
  let xlm = deploy_native_sac(&e);

  let carbonSac = Address::generate(&e);
  let sink = Address::generate(&e);
  let soroswapRouter = Address::generate(&e);

  let credit = create_credit_contract(
    &e,
    &admin,
    &initiative,
    &provider,
    &vendor,
    bucket,
    &xlm,
    &carbonSac,
    &sink,
    &soroswapRouter
  );

  // Views should all pass
  assert_eq!(credit.getAdmin(), admin);
  assert_eq!(credit.getBalance(), 0);
  assert_eq!(credit.getContractBalance(), 0);
  assert_eq!(credit.getBucket(), 200000000);
  assert_eq!(credit.getInitiative(), initiative);
  assert_eq!(credit.getMinimum(), 1000000);
  assert_eq!(credit.getProvider(), provider);
  assert_eq!(credit.getProviderFees(), 90);
  assert_eq!(credit.getVendor(), vendor);
  assert_eq!(credit.getVendorFees(), 10);
  assert_eq!(credit.getXLM(), xlm);
}


#[test]
fn test_donate() {
  let e = Env::default();
  e.mock_all_auths();

  let admin = Address::generate(&e);
  let bucket = 200_000_000;
  let donor_pubkey = "GA2H3SJYGIUG2DXXUZ7IN3LNO2AIMVWCDCL25PKQHKMC76OWW3HYQHY4";
  let donor = Address::from_str(&e, donor_pubkey);

  let initiative = String::from_str(&e, "30c0636f-b0f1-40d5-bb9c-a531dc4d69e2");
  let provider = Address::generate(&e);
  let vendor = Address::generate(&e);
  
  let xlm = deploy_native_sac(&e);
  let xlm_client = token::Client::new(&e, &xlm);

  let carbonSac = Address::generate(&e);
  let sink = Address::generate(&e);
  let soroswapRouter = Address::generate(&e);

  let credit = create_credit_contract(
    &e,
    &admin,
    &initiative,
    &provider,
    &vendor,
    bucket,
    &xlm,
    &carbonSac,
    &sink,
    &soroswapRouter
  );

  create_account_entry(&e, &donor_pubkey);
  assert_eq!(xlm_client.balance(&donor), 10_000_000_000);

  // Donate
  credit.donate(&donor, &100_000_000);

  assert_eq!(credit.getBalance(), 90_000_000); // amount - vendor fees
  assert_eq!(credit.getContractBalance(), 90_000_000);
  assert_eq!(xlm_client.balance(&donor), 9_900_000_000);
}

#[test]
fn test_set_sink_to_successor() {
  let e = Env::default();
  e.mock_all_auths();

  let admin = Address::generate(&e);
  let bucket = 200_000_000;

  let initiative = String::from_str(&e, "30c0636f-b0f1-40d5-bb9c-a531dc4d69e2");
  let provider = Address::generate(&e);
  let vendor = Address::generate(&e);
  
  let xlm = deploy_native_sac(&e);
  let carbonSac = Address::generate(&e);
  let (first_sink, last_sink) = create_sink_successors(&e);
  let soroswapRouter = Address::generate(&e);

  let credit = create_credit_contract(
    &e,
    &admin,
    &initiative,
    &provider,
    &vendor,
    bucket,
    &xlm,
    &carbonSac,
    &first_sink,
    &soroswapRouter
  );

  // set sink to successor; expect to go from first to last SinkContract
  assert_eq!(credit.getSink(), first_sink);
  credit.setSinkToSuccessor();
  let captured_events = e.events().all();
  assert_eq!(credit.getSink(), last_sink);

  // check if one event was published
  let expected_event = (
        credit.address,
        (
            Symbol::new(&e, "sink"),
            Symbol::new(&e, "change")
        ).into_val(&e),
        (first_sink, last_sink).into_val(&e)
    );
  assert_eq!(captured_events, vec![&e, expected_event]);
}