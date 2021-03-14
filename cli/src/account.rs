use clap::ArgMatches;
use keys::Address;
use proto::chain::transaction::Contract;
use proto::contract as contract_pb;

pub fn account(matches: &ArgMatches) -> Option<Contract> {
    match matches.subcommand() {
        ("create", Some(arg_matches)) => create(arg_matches),
        _ => unimplemented!(),
    }
}

fn create(matches: &ArgMatches) -> Option<Contract> {
    use proto::common::AccountType;

    let from: Address = matches.value_of("SENDER")?.parse().ok()?;
    let to: Address = matches.value_of("RECIPIENT")?.parse().ok()?;

    let account_type = match matches.value_of("type") {
        Some("normal") => AccountType::Normal,
        Some("assetissue") => AccountType::AssetIssue,
        Some("contract") => AccountType::Contract,
        _ => unreachable!("values restricted; qed"),
    };

    let inner = contract_pb::AccountCreateContract {
        owner_address: from.as_bytes().into(),
        account_address: to.as_bytes().into(),
        r#type: account_type as i32,
    };
    Some(inner.into())
}
