use crate::utils::utils::{is_testnet, load_core_contracts, setup_wallet};
use dotenv::dotenv;
use fuels::types::Identity;
use test_utils::interfaces::proxy::{proxy_abi, Proxy, State};

pub async fn transfer_proxy_ownership(new_owner: &str) {
    dotenv().ok();

    let wallet = setup_wallet().await;
    let address = wallet.address();
    println!("🔑 Wallet address: {}", address);

    let is_testnet = is_testnet(wallet.clone()).await;
    let core_contracts = load_core_contracts(wallet.clone(), is_testnet);

    println!(
        "Are you sure you want to transfer ownership to {}? (y/n)",
        new_owner
    );
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return;
    }

    // Convert string address to Identity
    let new_owner_identity = Identity::Address(new_owner.parse().expect("Invalid address format"));
    let proxy_owner_state = State::Initialized(new_owner_identity);

    // Transfer ownership for all proxy contracts
    let contracts_to_update = [
        (
            "Active Pool",
            core_contracts.active_pool.contract.contract_id(),
        ),
        (
            "Borrow Operations",
            core_contracts.borrow_operations.contract.contract_id(),
        ),
        ("USDF Token", core_contracts.usdf.contract.contract_id()),
        (
            "Stability Pool",
            core_contracts.stability_pool.contract.contract_id(),
        ),
        (
            "Protocol Manager",
            core_contracts.protocol_manager.contract.contract_id(),
        ),
        (
            "FPT Staking",
            core_contracts.fpt_staking.contract.contract_id(),
        ),
        (
            "Coll Surplus Pool",
            core_contracts.coll_surplus_pool.contract.contract_id(),
        ),
        (
            "Sorted Troves",
            core_contracts.sorted_troves.contract.contract_id(),
        ),
        (
            "Default Pool",
            core_contracts.default_pool.contract.contract_id(),
        ),
        ("FPT Token", core_contracts.fpt_token.contract.contract_id()),
        (
            "Community Issuance",
            core_contracts.community_issuance.contract.contract_id(),
        ),
        (
            "Vesting Contract",
            core_contracts.vesting_contract.contract.contract_id(),
        ),
    ];

    for (contract_name, contract_id) in contracts_to_update.iter() {
        let proxy_contract = Proxy::new(*contract_id, wallet.clone());
        if let Err(e) = proxy_abi::set_proxy_owner(&proxy_contract, proxy_owner_state.clone()).await
        {
            println!(
                "❌ Failed to transfer {} proxy ownership: {:?}",
                contract_name, e
            );
            std::process::exit(1);
        }
        println!(
            "✅ {} proxy ownership transferred successfully",
            contract_name
        );
    }

    // Transfer ownership for each asset's contracts
    for asset_contract in core_contracts.asset_contracts.iter() {
        let asset_specific_contracts = [
            ("Oracle", asset_contract.oracle.contract.contract_id()),
            (
                "Trove Manager",
                asset_contract.trove_manager.contract.contract_id(),
            ),
        ];

        println!(
            "\nTransferring ownership for Asset: {}",
            asset_contract.asset_id
        );
        for (contract_name, contract_id) in asset_specific_contracts.iter() {
            let proxy_contract = Proxy::new(*contract_id, wallet.clone());
            if let Err(e) =
                proxy_abi::set_proxy_owner(&proxy_contract, proxy_owner_state.clone()).await
            {
                println!(
                    "❌ Failed to transfer {} proxy ownership: {:?}",
                    contract_name, e
                );
                std::process::exit(1);
            }
            println!(
                "✅ {} proxy ownership transferred successfully",
                contract_name
            );
        }
    }

    println!("\n🎉 Proxy ownership transfer completed for all contracts");
}
