use deploy_scripts::{
    add_asset::add_asset,
    deploy::deployment::deploy,
    migrate_to_v2::migrate_to_v2,
    pause::{pause_protocol, unpause_protocol},
    sanity_check::sanity_check,
    transfer_ownership::transfer_owner,
};

#[tokio::main]
pub async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!(
            "Please specify 'deploy', 'add-asset <symbol>', 'pause', 'unpause', 'sanity-check', 'transfer-owner <address>', or 'migrate-v2'"
        );
        return;
    }

    match args[1].as_str() {
        "deploy" => deploy().await,
        "migrate-v2" => migrate_to_v2().await,
        "add-asset" => {
            if args.len() < 3 {
                println!("Please specify an asset symbol (e.g., 'add-asset ETH')");
                return;
            }
            add_asset(&args[2]).await
        },
        "pause" => pause_protocol().await,
        "unpause" => unpause_protocol().await,
        "sanity-check" => sanity_check().await,
        "transfer-owner" => {
            if args.len() < 3 {
                println!("Please specify the new owner address");
                return;
            }
            transfer_owner(&args[2]).await
        },
        _ => println!(
            "Invalid argument. Use 'deploy', 'add-asset <symbol>', 'pause', 'unpause', 'sanity-check', 'transfer-owner <address>', or 'migrate-v2'"
        ),
    }
}
