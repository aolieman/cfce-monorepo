extern crate std;

use soroban_sdk::{
  testutils::{
    Address as _,
    Events,
  },
  Address, Env, String, IntoVal, Symbol,
  vec,
};

use crate::tests::utils::{
  deploy_native_sac,
  create_account_entry,
  create_credit_contract,
  create_sink_successors,
  CreditTest
};

#[test]
fn test_views() {
  let test = CreditTest::setup();
  test.e.budget().reset_unlimited();

  // Views should all pass
  assert_eq!(test.credit.getAdmin(), test.admin);
  assert_eq!(test.credit.getBalance(), 0);
  assert_eq!(test.credit.getContractXLMBalance(), 0);
  assert_eq!(test.credit.getBucket(), test.bucket);
  assert_eq!(test.credit.getInitiative(), test.initiative);
  assert_eq!(test.credit.getMinimum(), 1000000);
  assert_eq!(test.credit.getProvider(), test.provider);
  assert_eq!(test.credit.getProviderFees(), 90);
  assert_eq!(test.credit.getVendor(), test.vendor);
  assert_eq!(test.credit.getVendorFees(), 10);
}


#[test]
fn test_donate() {
  let test = CreditTest::setup();
  test.e.budget().reset_unlimited();
  
  let donor_pubkey = "GDUY7J7A33TQWOSOQGDO776GGLM3UQERL4J3SPT56F6YS4ID7MLDERI4";
  let donor = Address::from_str(&test.e, donor_pubkey);

  create_account_entry(&test.e, &donor_pubkey, 10_000_000_000);
  assert_eq!(test.xlm.balance(&donor), 10_000_000_000);

  // Donate
  test.credit.donate(&donor, &100_000_000);

  assert_eq!(test.credit.getBalance(), 90_000_000); // amount - vendor fees
  assert_eq!(test.credit.getContractXLMBalance(), 90_000_000);
  assert_eq!(test.xlm.balance(&donor), 9_900_000_000);
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
  let usdc = Address::generate(&e);
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
    &usdc,
    &carbonSac,
    &first_sink,
    &soroswapRouter
  );

  // set sink to successor; expect to go from first to last SinkContract
  let (mut sink, _) = credit.getExternalContracts();
  assert_eq!(sink, first_sink);
  credit.setSinkToSuccessor();
  let captured_events = e.events().all();
  (sink, _) = credit.getExternalContracts();
  assert_eq!(sink, last_sink);

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


#[test]
fn test_token_swap_via_soroswap() {
  let test = CreditTest::setup();
  test.e.budget().reset_unlimited();

  let user = Address::generate(&test.e);
  test.xlm.transfer(&test.admin, &user, &10_000);

  assert_eq!(test.xlm.balance(&user), 10_000);
  assert_eq!(test.carbonSac.balance(&user), 0);

  test.credit.swap_tokens_via_soroswap(&user, &10_000);

  assert!(test.carbonSac.balance(&user) > 0);
}