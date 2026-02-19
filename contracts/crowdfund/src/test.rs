#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, Env, String};

use crate::{CrowdfundContract, CrowdfundContractClient};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Set up a fresh environment with a deployed crowdfund contract and a token.
fn setup_env() -> (Env, CrowdfundContractClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy the crowdfund contract.
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    // Create a token for contributions.
    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract_id.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    // Campaign creator.
    let creator = Address::generate(&env);

    // Mint tokens to the creator so the contract has something to work with.
    token_admin_client.mint(&creator, &10_000_000);

    (env, client, creator, token_address, token_admin.clone())
}

/// Helper to mint tokens to an arbitrary contributor.
fn mint_to(env: &Env, token_address: &Address, admin: &Address, to: &Address, amount: i128) {
    let admin_client = token::StellarAssetClient::new(env, token_address);
    admin_client.mint(to, &amount);
    let _ = admin;
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600; // 1 hour from now
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;

    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    assert_eq!(client.goal(), goal);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), min_contribution);
    assert_eq!(client.total_raised(), 0);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_double_initialize_panics() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;

    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution); // should panic
}

#[test]
fn test_contribute() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    client.contribute(&contributor, &500_000);

    assert_eq!(client.total_raised(), 500_000);
    assert_eq!(client.contribution(&contributor), 500_000);
}

#[test]
fn test_multiple_contributions() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 600_000);
    mint_to(&env, &token_address, &admin, &bob, 400_000);

    client.contribute(&alice, &600_000);
    client.contribute(&bob, &400_000);

    assert_eq!(client.total_raised(), 1_000_000);
    assert_eq!(client.contribution(&alice), 600_000);
    assert_eq!(client.contribution(&bob), 400_000);
}

#[test]
#[should_panic(expected = "campaign has ended")]
fn test_contribute_after_deadline_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 100;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    // Fast-forward past the deadline.
    env.ledger().set_timestamp(deadline + 1);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    client.contribute(&contributor, &500_000); // should panic
}

#[test]
fn test_withdraw_after_goal_met() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    assert_eq!(client.total_raised(), goal);

    // Move past deadline.
    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    // After withdrawal, total_raised resets to 0.
    assert_eq!(client.total_raised(), 0);

    // Creator should have received the funds.
    let token_client = token::Client::new(&env, &token_address);
    assert_eq!(token_client.balance(&creator), 10_000_000 + 1_000_000);
}

#[test]
#[should_panic(expected = "campaign is still active")]
fn test_withdraw_before_deadline_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    client.withdraw(); // should panic — deadline not passed
}

#[test]
#[should_panic(expected = "goal not reached")]
fn test_withdraw_goal_not_reached_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    // Move past deadline, but goal not met.
    env.ledger().set_timestamp(deadline + 1);

    client.withdraw(); // should panic
}

#[test]
fn test_refund_when_goal_not_met() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 300_000);
    mint_to(&env, &token_address, &admin, &bob, 200_000);

    client.contribute(&alice, &300_000);
    client.contribute(&bob, &200_000);

    // Move past deadline — goal not met.
    env.ledger().set_timestamp(deadline + 1);

    client.refund();

    // Both contributors should get their tokens back.
    let token_client = token::Client::new(&env, &token_address);
    assert_eq!(token_client.balance(&alice), 300_000);
    assert_eq!(token_client.balance(&bob), 200_000);
    assert_eq!(client.total_raised(), 0);
}

#[test]
#[should_panic(expected = "goal was reached; use withdraw instead")]
fn test_refund_when_goal_reached_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.refund(); // should panic — goal was met
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_double_withdraw_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();
    client.withdraw(); // should panic — status is Successful
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_double_refund_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 500_000);
    client.contribute(&alice, &500_000);

    env.ledger().set_timestamp(deadline + 1);

    client.refund();
    client.refund(); // should panic — status is Refunded
}

#[test]
fn test_cancel_with_no_contributions() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    client.cancel();

    assert_eq!(client.total_raised(), 0);
}

#[test]
fn test_cancel_with_contributions() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &alice, 300_000);
    mint_to(&env, &token_address, &admin, &bob, 200_000);

    client.contribute(&alice, &300_000);
    client.contribute(&bob, &200_000);

    client.cancel();

    let token_client = token::Client::new(&env, &token_address);
    assert_eq!(token_client.balance(&alice), 300_000);
    assert_eq!(token_client.balance(&bob), 200_000);
    assert_eq!(client.total_raised(), 0);
}

#[test]
#[should_panic]
fn test_cancel_by_non_creator_panics() {
    let env = Env::default();
    let contract_id = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract_id.address();

    let creator = Address::generate(&env);
    let non_creator = Address::generate(&env);

    env.mock_all_auths();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    env.mock_all_auths_allowing_non_root_auth();
    env.set_auths(&[]);
    
    client.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &non_creator,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "cancel",
            args: soroban_sdk::vec![&env],
            sub_invokes: &[],
        },
    }]);

    client.cancel();
}

// ── Minimum Contribution Tests ─────────────────────────────────────────────

#[test]
#[should_panic(expected = "amount below minimum")]
fn test_contribute_below_minimum_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 5_000);

    client.contribute(&contributor, &5_000); // should panic
}

#[test]
fn test_contribute_exact_minimum() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 10_000);

    client.contribute(&contributor, &10_000);

    assert_eq!(client.total_raised(), 10_000);
    assert_eq!(client.contribution(&contributor), 10_000);
}

#[test]
fn test_contribute_above_minimum() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 10_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 50_000);

    client.contribute(&contributor, &50_000);

    assert_eq!(client.total_raised(), 50_000);
    assert_eq!(client.contribution(&contributor), 50_000);
}

// ── Metadata Update Tests ──────────────────────────────────────────────────

/// Helper to create a String from a &str.
fn str_to_string(env: &Env, s: &str) -> String {
    String::from_slice(env, s)
}

#[test]
fn test_update_title() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let new_title = str_to_string(&env, "New Campaign Title");
    client.update_metadata(&creator, &Some(new_title.clone()), &None, &None);

    assert_eq!(client.title(), new_title);
}

#[test]
fn test_update_description() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let new_description = str_to_string(&env, "This is an updated description for the campaign.");
    client.update_metadata(&creator, &None, &Some(new_description.clone()), &None);

    assert_eq!(client.description(), new_description);
}

#[test]
fn test_update_socials() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    let new_socials = str_to_string(&env, "https://twitter.com/campaign");
    client.update_metadata(&creator, &None, &None, &Some(new_socials.clone()));

    assert_eq!(client.socials(), new_socials);
}

#[test]
fn test_partial_update() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    // First set all fields.
    let initial_title = str_to_string(&env, "Initial Title");
    let initial_description = str_to_string(&env, "Initial Description");
    let initial_socials = str_to_string(&env, "https://initial.social");
    client.update_metadata(
        &creator,
        &Some(initial_title.clone()),
        &Some(initial_description.clone()),
        &Some(initial_socials.clone()),
    );

    assert_eq!(client.title(), initial_title);
    assert_eq!(client.description(), initial_description);
    assert_eq!(client.socials(), initial_socials);

    // Now update only title and socials, leaving description unchanged.
    let new_title = str_to_string(&env, "Updated Title");
    let new_socials = str_to_string(&env, "https://updated.social");
    client.update_metadata(
        &creator,
        &Some(new_title.clone()),
        &None,
        &Some(new_socials.clone()),
    );

    // Title and socials should be updated.
    assert_eq!(client.title(), new_title);
    assert_eq!(client.socials(), new_socials);
    // Description should remain unchanged.
    assert_eq!(client.description(), initial_description);
}

#[test]
#[should_panic(expected = "campaign is not active")]
fn test_update_metadata_when_not_active_panics() {
    let (env, client, creator, token_address, admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    // Make a contribution to meet the goal.
    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    // Move past deadline and withdraw to set status to Successful.
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    // Try to update metadata - should panic because status is not Active.
    let new_title = str_to_string(&env, "New Title");
    client.update_metadata(&creator, &Some(new_title), &None, &None);
}

#[test]
#[should_panic(expected = "not authorized")]
fn test_update_metadata_by_non_creator_panics() {
    let (env, client, creator, token_address, _admin) = setup_env();

    let deadline = env.ledger().timestamp() + 3600;
    let goal: i128 = 1_000_000;
    let min_contribution: i128 = 1_000;
    client.initialize(&creator, &token_address, &goal, &deadline, &min_contribution);

    // Try to update metadata from a non-creator address - should panic.
    let non_creator = Address::generate(&env);
    let new_title = str_to_string(&env, "New Title");
    client.update_metadata(&non_creator, &Some(new_title), &None, &None);
}
