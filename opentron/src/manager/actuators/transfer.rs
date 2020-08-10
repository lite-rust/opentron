use std::convert::TryFrom;

use ::keys::Address;
use proto2::chain::transaction::Result as TransactionResult;
use proto2::common::AccountType;
use proto2::contract as contract_pb;
use proto2::state::Account;
use state::keys;

use super::super::executor::TransactionContext;
use super::super::Manager;
use super::BuiltinContractExecutorExt;

impl BuiltinContractExecutorExt for contract_pb::TransferContract {
    fn validate(&self, manager: &Manager, ctx: &mut TransactionContext) -> Result<(), String> {
        let state_db = &manager.state_db;

        let owner_address = Address::try_from(&self.owner_address).map_err(|_| "invalid owner_address")?;
        let to_address = Address::try_from(&self.to_address).map_err(|_| "invalid to_address")?;

        let mut fee = self.fee();

        if owner_address == to_address {
            return Err("cannot transfer to oneself".into());
        }

        if self.amount <= 0 {
            return Err("transfer amount must be greater than 0".into());
        }

        let owner_acct = state_db
            .get(&keys::Account(owner_address))
            .map_err(|_| "error while querying db")?;

        if owner_acct.is_none() {
            return Err("owner account is not on chain".into());
        }
        let owner_acct = owner_acct.unwrap();

        let to_acct = state_db
            .get(&keys::Account(to_address))
            .map_err(|_| "error while querying db")?;

        if to_acct.is_none() {
            ctx.new_account_created = true;
            // NOTE: CreateNewAccountFeeInSystemContract is 0, account creation fee is handled by BandwidthProcessor.
            fee += state_db.must_get(&keys::ChainParameter::CreateNewAccountFeeInSystemContract);
        } else if to_acct.as_ref().unwrap().r#type == AccountType::Contract as i32 &&
            state_db.must_get(&keys::ChainParameter::ForbidTransferToContract) == 1
        {
            return Err("cannot transfer to a smart contract address".into());
        }

        if let Some(spend) = self.amount.checked_add(fee) {
            if owner_acct.balance < spend {
                return Err("insufficient balance".into());
            }
        } else {
            return Err("math overflow".into());
        }

        if let Some(to_acct) = to_acct {
            if to_acct.balance.checked_add(self.amount).is_none() {
                return Err("math overflow".into());
            }
        }

        ctx.contract_fee = fee;

        Ok(())
    }

    fn execute(&self, manager: &mut Manager, ctx: &mut TransactionContext) -> Result<TransactionResult, String> {
        let owner_address = Address::try_from(&self.owner_address).unwrap();
        let to_address = Address::try_from(&self.to_address).unwrap();

        let mut owner_acct = manager.state_db.must_get(&keys::Account(owner_address));

        let fee = ctx.contract_fee;

        let mut to_acct = manager
            .state_db
            .get(&keys::Account(to_address))
            .map_err(|e| format!("state-db error: {:?}", e))?
            .unwrap_or_else(|| Account::new(ctx.block_header.timestamp()));

        if fee != 0 {
            owner_acct.adjust_balance(-fee).unwrap();
            manager.add_to_blackhole(fee).unwrap();
        }

        owner_acct.adjust_balance(-self.amount).unwrap();
        to_acct.adjust_balance(self.amount).unwrap();

        manager
            .state_db
            .put_key(keys::Account(owner_address), owner_acct)
            .map_err(|e| e.to_string())?;
        manager
            .state_db
            .put_key(keys::Account(to_address), to_acct)
            .map_err(|e| e.to_string())?;

        // NOTE: Default status is a `SUCCESS`. Although it's a bad design, OpenTron cannot change this.
        // This is the the `ret` field of a transaction, and is saved in chain-db, participating in MerkleTree.
        Ok(TransactionResult::default())
    }
}
