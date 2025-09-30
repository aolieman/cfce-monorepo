#![no_std]
#![allow(non_snake_case)]

mod admin;
mod contract;
mod events;
mod storage;
mod tests;

pub use crate::contract::CreditsClient;

mod sink_contract {
  soroban_sdk::contractimport!(
    file = "external-contracts/sink-carbon_v0.3.0.wasm"
  );
}

mod soroswap_router {
  soroban_sdk::contractimport!(
    file = "external-contracts/soroswap_router_contract.wasm"
  );
}

mod soroswap_factory {
  soroban_sdk::contractimport!(
    file = "external-contracts/soroswap_factory_contract.wasm"
  );
}

mod soroswap_pair {
  soroban_sdk::contractimport!(
    file = "external-contracts/soroswap_pair_contract.wasm"
  );
}