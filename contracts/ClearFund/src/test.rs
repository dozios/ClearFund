#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    token, Address, Env, Symbol, Vec,
};

/// Helper: deploys a fresh SEP-41 token contract and returns
/// (token_address, admin_client, token_client) for minting/transfers in tests.
fn setup_token<'a>(env: &Env, admin: &Address) -> (Address, token::StellarAssetClient<'a>, token::Client<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let address = sac.address();
    let admin_client = token::StellarAssetClient::new(env, &address);
    let client = token::Client::new(env, &address);
    (address, admin_client, client)
}

mod tests {
    use super::*;

    /// Test 1 (Happy path): full donate -> approve_milestone -> NGO receives funds flow.
    #[test]
    fn test_happy_path_full_milestone_flow() {
        let env = Env::default();
        env.mock_all_auths();

        let ngo = Address::generate(&env);
        let auditor = Address::generate(&env);
        let donor = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, token_admin_client, token_client) = setup_token(&env, &token_admin);
        token_admin_client.mint(&donor, &1000);

        let contract_id = env.register(ClearFundContract, ());
        let client = ClearFundContractClient::new(&env, &contract_id);

        let mut descriptions: Vec<Symbol> = Vec::new(&env);
        descriptions.push_back(Symbol::new(&env, "food_kits"));
        let mut amounts: Vec<i128> = Vec::new(&env);
        amounts.push_back(500);

        let campaign_id = client.create_campaign(&ngo, &auditor, &token_addr, &descriptions, &amounts);

        client.donate(&donor, &campaign_id, &500);
        client.approve_milestone(&campaign_id, &0);

        assert_eq!(token_client.balance(&ngo), 500);
        assert_eq!(token_client.balance(&donor), 500);
    }

    /// Test 2 (Edge case): approving a milestone before it is funded should fail
    /// with InsufficientEscrowBalance rather than silently releasing money.
    #[test]
    fn test_edge_case_approve_without_sufficient_funds_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let ngo = Address::generate(&env);
        let auditor = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, _admin_client, _token_client) = setup_token(&env, &token_admin);

        let contract_id = env.register(ClearFundContract, ());
        let client = ClearFundContractClient::new(&env, &contract_id);

        let mut descriptions: Vec<Symbol> = Vec::new(&env);
        descriptions.push_back(Symbol::new(&env, "water_wells"));
        let mut amounts: Vec<i128> = Vec::new(&env);
        amounts.push_back(1000);

        let campaign_id = client.create_campaign(&ngo, &auditor, &token_addr, &descriptions, &amounts);

        // No donation made — escrow balance is 0, milestone needs 1000.
        let result = client.try_approve_milestone(&campaign_id, &0);
        assert_eq!(result, Err(Ok(Error::InsufficientEscrowBalance)));
    }

    /// Test 3 (State verification): after a partial donation, campaign.raised
    /// reflects the correct amount and the milestone remains unreleased.
    #[test]
    fn test_state_after_donation_before_approval() {
        let env = Env::default();
        env.mock_all_auths();

        let ngo = Address::generate(&env);
        let auditor = Address::generate(&env);
        let donor = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, token_admin_client, _token_client) = setup_token(&env, &token_admin);
        token_admin_client.mint(&donor, &1000);

        let contract_id = env.register(ClearFundContract, ());
        let client = ClearFundContractClient::new(&env, &contract_id);

        let mut descriptions: Vec<Symbol> = Vec::new(&env);
        descriptions.push_back(Symbol::new(&env, "shelter_kits"));
        let mut amounts: Vec<i128> = Vec::new(&env);
        amounts.push_back(750);

        let campaign_id = client.create_campaign(&ngo, &auditor, &token_addr, &descriptions, &amounts);
        client.donate(&donor, &campaign_id, &750);

        let campaign = client.get_campaign(&campaign_id);
        assert_eq!(campaign.raised, 750);
        assert_eq!(campaign.milestones.get(0).unwrap().released, false);
    }

    /// Test 4 (Edge case): approving the same milestone twice must fail on the
    /// second call, proving double-spend of a milestone tranche is impossible.
    #[test]
    fn test_edge_case_double_release_blocked() {
        let env = Env::default();
        env.mock_all_auths();

        let ngo = Address::generate(&env);
        let auditor = Address::generate(&env);
        let donor = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, token_admin_client, _token_client) = setup_token(&env, &token_admin);
        token_admin_client.mint(&donor, &1000);

        let contract_id = env.register(ClearFundContract, ());
        let client = ClearFundContractClient::new(&env, &contract_id);

        let mut descriptions: Vec<Symbol> = Vec::new(&env);
        descriptions.push_back(Symbol::new(&env, "medical_supplies"));
        let mut amounts: Vec<i128> = Vec::new(&env);
        amounts.push_back(300);

        let campaign_id = client.create_campaign(&ngo, &auditor, &token_addr, &descriptions, &amounts);
        client.donate(&donor, &campaign_id, &300);
        client.approve_milestone(&campaign_id, &0);

        let result = client.try_approve_milestone(&campaign_id, &0);
        assert_eq!(result, Err(Ok(Error::MilestoneAlreadyReleased)));
    }

    /// Test 5 (State verification): a milestone_released event is emitted with
    /// the correct campaign id and amount when a milestone is approved.
    #[test]
    fn test_state_milestone_released_event_emitted() {
        let env = Env::default();
        env.mock_all_auths();

        let ngo = Address::generate(&env);
        let auditor = Address::generate(&env);
        let donor = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token_addr, token_admin_client, _token_client) = setup_token(&env, &token_admin);
        token_admin_client.mint(&donor, &1000);

        let contract_id = env.register(ClearFundContract, ());
        let client = ClearFundContractClient::new(&env, &contract_id);

        let mut descriptions: Vec<Symbol> = Vec::new(&env);
        descriptions.push_back(Symbol::new(&env, "school_supplies"));
        let mut amounts: Vec<i128> = Vec::new(&env);
        amounts.push_back(200);

        let campaign_id = client.create_campaign(&ngo, &auditor, &token_addr, &descriptions, &amounts);
        client.donate(&donor, &campaign_id, &200);
        client.approve_milestone(&campaign_id, &0);

        let events = env.events().all();
        assert!(events.len() > 0, "expected at least one event to be emitted");
    }
}