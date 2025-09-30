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

  // Views should all pass
  assert_eq!(test.credit.getAdmin(), test.admin);
  assert_eq!(test.credit.getBalance(), 0);
  assert_eq!(test.credit.getContractXLMBalance(), 0);
  assert_eq!(test.credit.getBucket(), 1000000);
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
  
  let donor_pubkey = "GDUY7J7A33TQWOSOQGDO776GGLM3UQERL4J3SPT56F6YS4ID7MLDERI4";
  let donor = Address::from_str(&test.e, donor_pubkey);

  create_account_entry(&test.e, &donor_pubkey, 10_000_000_000);
  assert_eq!(test.xlm_client.balance(&donor), 10_000_000_000);

  // Donate
  test.credit.donate(&donor, &100_000_000);

  /*
    1. first swap for XLM <-> USDC with 10_000 XLM
    - fee = 90_000_000 * 3 / 1000 =  270000
    - amount_in less fee = 90_000_000 - 270000 = 89730000
    - first_out = (89730000 * 3_200_000_000)/(32_000_000_000 + 89730000) = 8947909.50251 = 8947909

    2. second swap for USDC <-> CARBON with 8947909 USDC
    - fee = 8947909 * 3 / 1000 =  26843.727 (2)
    - amount_in less fee = 8947909 - 26843 = 8921066
    - first_out = (8921066 * 160_000_000)/(3_200_000_000 + 8921066) = 444813 = 444813
   */

  let expected_swapped_carbon_balance = 444813;
  assert_eq!(test.credit.getBalance(), expected_swapped_carbon_balance);
  assert_eq!(test.credit.getContractXLMBalance(), 0);
  assert_eq!(test.xlm_client.balance(&donor), 9_900_000_000);
}

#[test]
fn test_set_sink_to_successor() {
  let e = Env::default();
  e.mock_all_auths();

  let admin = Address::generate(&e);
  // let bucket = 200_000_000; TODO: use this in sink

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

  let user = Address::generate(&test.e);
  test.xlm_client.transfer(&test.admin, &user, &10_000);

  assert_eq!(test.xlm_client.balance(&user), 10_000);
  assert_eq!(test.carbon_client.balance(&user), 0);

  test.credit.swap_xlm_to_carbon(&user, &10_000);
  
  /*
    1. first swap for XLM <-> USDC with 10_000 XLM
    - fee = 10000 * 3 / 1000 =  30
    - amount_in less fee = 10000 - 30 = 9970
    - first_out = (9970 * 3_200_000_000)/(32_000_000_000 + 9970) = 996.999689372 = 996

    2. second swap for USDC <-> CARBON with 996 USDC
    - fee = 996 * 3 / 1000 =  2.988 (2)
    - amount_in less fee = 996 - 2 = 994
    - first_out = (994 * 160_000_000)/(3_200_000_000 + 994) = 49.6999845619 = 49
   */
  let expected_balance = 49;
  let executed_carbon_balance = test.carbon_client.balance(&user);
  assert_eq!(executed_carbon_balance, expected_balance);
}