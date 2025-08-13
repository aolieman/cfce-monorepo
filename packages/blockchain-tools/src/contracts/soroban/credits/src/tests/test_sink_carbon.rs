use soroban_sdk::{
  testutils::{Address as _, MockAuth, MockAuthInvoke},
  Address, IntoVal,
};
use crate::tests::utils::set_up_contracts_and_funder;
  
#[test]
#[should_panic = "HostError: Error(Auth, InvalidAction)"]
fn test_set_successor_unauthorized() {
  let setup = set_up_contracts_and_funder(0, None);
  let client = setup.sink_client;

  // it should fail because the call lacks admin auth
  client.set_contract_successor(&client.address);
}

#[test]
fn test_set_and_get_successor() {
  let setup = set_up_contracts_and_funder(0, None);
  let client = setup.sink_client;
  let admin = setup.carbonsink_issuer;
  let new_contract = Address::generate(&setup.env);

  // set new contract successor
  client
      .mock_auths(&[MockAuth {
          address: &admin,
          invoke: &MockAuthInvoke {
              contract: &client.address,
              fn_name: "set_contract_successor",
              args: (&new_contract,).into_val(&setup.env),
              sub_invokes: &[],
          },
      }])
      .set_contract_successor(&new_contract);

  let successor: Address = client.get_contract_successor();
  assert_eq!(successor, new_contract);
}

#[test]
fn test_set_and_get_successor_multiple() {
  let first_setup = set_up_contracts_and_funder(0, None);
  let second_setup = set_up_contracts_and_funder(0, None);
  let third_setup = set_up_contracts_and_funder(0, None);

  // set successor for third sink carbon
  let mut successor: Address = second_setup.contract_id;
  third_setup.sink_client
    .mock_auths(&[MockAuth {
        address: &third_setup.carbonsink_issuer,
        invoke: &MockAuthInvoke {
            contract: &third_setup.sink_client.address,
            fn_name: "set_contract_successor",
            args: (&successor,).into_val(&third_setup.env),
            sub_invokes: &[],
        },
    }])
    .set_contract_successor(&successor);
  assert_eq!(third_setup.sink_client.get_contract_successor(), successor);

  // set successor for second sink carbon
  successor = first_setup.contract_id;
  second_setup.sink_client
    .mock_auths(&[MockAuth {
        address: &second_setup.carbonsink_issuer,
        invoke: &MockAuthInvoke {
            contract: &second_setup.sink_client.address,
            fn_name: "set_contract_successor",
            args: (&successor,).into_val(&second_setup.env),
            sub_invokes: &[],
        },
    }])
    .set_contract_successor(&successor);
  assert_eq!(second_setup.sink_client.get_contract_successor(), successor);

}