// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::AccountData;
use admission_control_proto::{
    proto::{
        admission_control::{
            SubmitTransactionRequest, SubmitTransactionResponse as ProtoSubmitTransactionResponse,
        },
        admission_control_grpc::AdmissionControlClient,
    },
    AdmissionControlStatus, SubmitTransactionResponse,
};
use failure::prelude::*;
use futures::Future;
use grpcio::{CallOption, ChannelBuilder, EnvBuilder};
use logger::prelude::*;
use proto_conv::{FromProto, IntoProto};
use std::sync::Arc;
use types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    account_config::get_account_resource_or_default,
    account_state_blob::{AccountStateBlob, AccountStateWithProof},
    contract_event::{ContractEvent, EventWithProof},
    get_with_proof::{
        RequestItem, ResponseItem, UpdateToLatestLedgerRequest, UpdateToLatestLedgerResponse,
    },
    transaction::{SignedTransaction, Version},
    validator_verifier::ValidatorVerifier,
    vm_error::{VMStatus, VMValidationStatus},
};

const MAX_GRPC_RETRY_COUNT: u64 = 1;

/// Struct holding dependencies of client.
pub struct GRPCClient {
    client: AdmissionControlClient,
    validator_verifier: Arc<ValidatorVerifier>,
}

impl GRPCClient {
    /// Construct a new Client instance.
    pub fn new(host: &str, port: &str, validator_verifier: Arc<ValidatorVerifier>) -> Result<Self> {
        let conn_addr = format!("{}:{}", host, port);

        // Create a GRPC client
        let env = Arc::new(EnvBuilder::new().name_prefix("grpc-client-").build());
        let ch = ChannelBuilder::new(env).connect(&conn_addr);
        let client = AdmissionControlClient::new(ch);

        Ok(GRPCClient {
            client,
            validator_verifier,
        })
    }

    /// Submits a transaction and bumps the sequence number for the sender
    pub fn submit_transaction(
        &self,
        sender_account: &mut AccountData,
        req: &SubmitTransactionRequest,
    ) -> Result<()> {
        let mut resp = self.submit_transaction_opt(req);

        let mut try_cnt = 0_u64;
        while Self::need_to_retry(&mut try_cnt, &resp) {
            resp = self.submit_transaction_opt(&req);
        }

        let completed_resp = SubmitTransactionResponse::from_proto(resp?)?;

        if let Some(ac_status) = completed_resp.ac_status {
            if ac_status == AdmissionControlStatus::Accepted {
                // Bump up sequence_number if transaction is accepted.
                sender_account.sequence_number += 1;
            } else {
                bail!("Transaction failed with AC status: {:?}", ac_status,);
            }
        } else if let Some(vm_error) = completed_resp.vm_error {
            if vm_error == VMStatus::Validation(VMValidationStatus::SequenceNumberTooOld) {
                sender_account.sequence_number =
                    self.get_sequence_number(sender_account.address)?;
                bail!(
                    "Transaction failed with vm status: {:?}, please retry your transaction.",
                    vm_error
                );
            }
            bail!("Transaction failed with vm status: {:?}", vm_error);
        } else if let Some(mempool_error) = completed_resp.mempool_error {
            bail!(
                "Transaction failed with mempool status: {:?}",
                mempool_error,
            );
        } else {
            bail!(
                "Malformed SubmitTransactionResponse which has no status set, {:?}",
                completed_resp,
            );
        }
        Ok(())
    }

    /// Async version of submit_transaction
    pub fn submit_transaction_async(
        &self,
        req: &SubmitTransactionRequest,
    ) -> Result<(impl Future<Item = SubmitTransactionResponse, Error = failure::Error>)> {
        let resp = self
            .client
            .submit_transaction_async_opt(&req, Self::get_default_grpc_call_option())?
            .then(|proto_resp| {
                let ret = SubmitTransactionResponse::from_proto(proto_resp?)?;
                Ok(ret)
            });
        Ok(resp)
    }

    fn submit_transaction_opt(
        &self,
        resp: &SubmitTransactionRequest,
    ) -> Result<ProtoSubmitTransactionResponse> {
        Ok(self
            .client
            .submit_transaction_opt(resp, Self::get_default_grpc_call_option())?)
    }

    fn get_with_proof_async(
        &self,
        requested_items: Vec<RequestItem>,
    ) -> Result<impl Future<Item = UpdateToLatestLedgerResponse, Error = failure::Error>> {
        let req = UpdateToLatestLedgerRequest::new(0, requested_items.clone());
        debug!("get_with_proof with request: {:?}", req);
        let proto_req = req.clone().into_proto();
        let arc_validator_verifier: Arc<ValidatorVerifier> = Arc::clone(&self.validator_verifier);
        let ret = self
            .client
            .update_to_latest_ledger_async_opt(&proto_req, Self::get_default_grpc_call_option())?
            .then(move |get_with_proof_resp| {
                // TODO: Cache/persist client_known_version to work with validator set change when
                // the feature is available.

                let resp = UpdateToLatestLedgerResponse::from_proto(get_with_proof_resp?)?;
                resp.verify(arc_validator_verifier, &req)?;
                Ok(resp)
            });
        Ok(ret)
    }

    fn need_to_retry<T>(try_cnt: &mut u64, ret: &Result<T>) -> bool {
        if *try_cnt <= MAX_GRPC_RETRY_COUNT {
            *try_cnt += 1;
            if let Err(error) = ret {
                if let Some(grpc_error) = error.downcast_ref::<grpcio::Error>() {
                    if let grpcio::Error::RpcFailure(grpc_rpc_failure) = grpc_error {
                        // Only retry when the connection is down to make sure we won't
                        // send one txn twice.
                        return grpc_rpc_failure.status == grpcio::RpcStatusCode::Unavailable;
                    }
                }
            }
        }
        false
    }
    /// Sync version of get_with_proof
    pub fn get_with_proof_sync(
        &self,
        requested_items: Vec<RequestItem>,
    ) -> Result<UpdateToLatestLedgerResponse> {
        let mut resp: Result<UpdateToLatestLedgerResponse> =
            self.get_with_proof_async(requested_items.clone())?.wait();
        let mut try_cnt = 0_u64;

        while Self::need_to_retry(&mut try_cnt, &resp) {
            resp = self.get_with_proof_async(requested_items.clone())?.wait();
        }

        Ok(resp?)
    }

    fn get_balances_async(
        &self,
        addresses: &[AccountAddress],
    ) -> Result<impl Future<Item = Vec<u64>, Error = failure::Error>> {
        let requests = addresses
            .iter()
            .map(|addr| RequestItem::GetAccountState { address: *addr })
            .collect::<Vec<_>>();

        let num_addrs = addresses.len();
        let get_with_proof_resp = self.get_with_proof_async(requests)?;
        Ok(get_with_proof_resp.then(move |get_with_proof_resp| {
            let rust_resp = get_with_proof_resp?;
            if rust_resp.response_items.len() != num_addrs {
                bail!("Server returned wrong number of responses");
            }

            let mut balances = vec![];
            for value_with_proof in rust_resp.response_items {
                debug!("get_balance response is: {:?}", value_with_proof);
                match value_with_proof {
                    ResponseItem::GetAccountState {
                        account_state_with_proof,
                    } => {
                        let balance =
                            get_account_resource_or_default(&account_state_with_proof.blob)?
                                .balance();
                        balances.push(balance);
                    }
                    _ => bail!(
                        "Incorrect type of response returned: {:?}",
                        value_with_proof
                    ),
                }
            }
            Ok(balances)
        }))
    }

    pub(crate) fn get_balance(&self, address: AccountAddress) -> Result<u64> {
        let mut ret = self.get_balances_async(&[address])?.wait();
        let mut try_cnt = 0_u64;
        while Self::need_to_retry(&mut try_cnt, &ret) {
            ret = self.get_balances_async(&[address])?.wait();
        }

        ret?.pop()
            .ok_or_else(|| format_err!("Account is not available!"))
    }

    /// Get the latest account sequence number for the account specified.
    pub fn get_sequence_number(&self, address: AccountAddress) -> Result<u64> {
        Ok(get_account_resource_or_default(&self.get_account_blob(address)?.0)?.sequence_number())
    }

    /// Get the latest account state blob from validator.
    pub fn get_account_blob(
        &self,
        address: AccountAddress,
    ) -> Result<(Option<AccountStateBlob>, Version)> {
        let req_item = RequestItem::GetAccountState { address };

        let mut response = self.get_with_proof_sync(vec![req_item])?;
        let account_state_with_proof = response
            .response_items
            .remove(0)
            .into_get_account_state_response()?;

        Ok((
            account_state_with_proof.blob,
            response.ledger_info_with_sigs.ledger_info().version(),
        ))
    }

    /// Get transaction from validator by account and sequence number.
    pub fn get_txn_by_acc_seq(
        &self,
        account: AccountAddress,
        sequence_number: u64,
        fetch_events: bool,
    ) -> Result<Option<(SignedTransaction, Option<Vec<ContractEvent>>)>> {
        let req_item = RequestItem::GetAccountTransactionBySequenceNumber {
            account,
            sequence_number,
            fetch_events,
        };

        let mut response = self.get_with_proof_sync(vec![req_item])?;
        let (signed_txn_with_proof, _) = response
            .response_items
            .remove(0)
            .into_get_account_txn_by_seq_num_response()?;

        Ok(signed_txn_with_proof.map(|t| (t.signed_transaction, t.events)))
    }

    /// Get transactions in range (start_version..start_version + limit - 1) from validator.
    pub fn get_txn_by_range(
        &self,
        start_version: u64,
        limit: u64,
        fetch_events: bool,
    ) -> Result<Vec<(SignedTransaction, Option<Vec<ContractEvent>>)>> {
        // Make the request.
        let req_item = RequestItem::GetTransactions {
            start_version,
            limit,
            fetch_events,
        };
        let mut response = self.get_with_proof_sync(vec![req_item])?;
        let txn_list_with_proof = response
            .response_items
            .remove(0)
            .into_get_transactions_response()?;

        // Transform the response.
        let num_txns = txn_list_with_proof.transaction_and_infos.len();
        let event_lists = txn_list_with_proof
            .events
            .map(|event_lists| event_lists.into_iter().map(Some).collect())
            .unwrap_or_else(|| vec![None; num_txns]);

        let res = itertools::zip_eq(txn_list_with_proof.transaction_and_infos, event_lists)
            .map(|((signed_txn, _), events)| (signed_txn, events))
            .collect();
        Ok(res)
    }

    /// Get event by access path from validator. AccountStateWithProof will be returned if
    /// 1. No event is available. 2. Ascending and available event number < limit.
    /// 3. Descending and start_seq_num > latest account event sequence number.
    pub fn get_events_by_access_path(
        &self,
        access_path: AccessPath,
        start_event_seq_num: u64,
        ascending: bool,
        limit: u64,
    ) -> Result<(Vec<EventWithProof>, Option<AccountStateWithProof>)> {
        let req_item = RequestItem::GetEventsByEventAccessPath {
            access_path,
            start_event_seq_num,
            ascending,
            limit,
        };

        let mut response = self.get_with_proof_sync(vec![req_item])?;
        let value_with_proof = response.response_items.remove(0);
        match value_with_proof {
            ResponseItem::GetEventsByEventAccessPath {
                events_with_proof,
                proof_of_latest_event,
            } => Ok((events_with_proof, proof_of_latest_event)),
            _ => bail!(
                "Incorrect type of response returned: {:?}",
                value_with_proof
            ),
        }
    }

    fn get_default_grpc_call_option() -> CallOption {
        CallOption::default()
            .wait_for_ready(true)
            .timeout(std::time::Duration::from_millis(5000))
    }
}

#[cfg(test)]
mod tests {
    use crate::client_proxy::{AddressAndIndex, ClientProxy};
    use config::trusted_peers::TrustedPeersConfigHelpers;
    use libra_wallet::io_utils;
    use tempfile::NamedTempFile;

    pub fn generate_accounts_from_wallet(count: usize) -> (ClientProxy, Vec<AddressAndIndex>) {
        let mut accounts = Vec::new();
        accounts.reserve(count);
        let file = NamedTempFile::new().unwrap();
        let mnemonic_path = file.into_temp_path().to_str().unwrap().to_string();
        let trust_peer_file = NamedTempFile::new().unwrap();
        let (_, trust_peer_config) = TrustedPeersConfigHelpers::get_test_config(1, None);
        let trust_peer_path = trust_peer_file.into_temp_path();
        trust_peer_config.save_config(&trust_peer_path);

        let val_set_file = trust_peer_path.to_str().unwrap().to_string();

        // We don't need to specify host/port since the client won't be used to connect, only to
        // generate random accounts
        let mut client_proxy = ClientProxy::new(
            "", /* host */
            "", /* port */
            &val_set_file,
            &"",
            None,
            Some(mnemonic_path),
        )
        .unwrap();
        for _ in 0..count {
            accounts.push(client_proxy.create_next_account(&["c"]).unwrap());
        }

        (client_proxy, accounts)
    }

    #[test]
    fn test_generate() {
        let num = 1;
        let (_, accounts) = generate_accounts_from_wallet(num);
        assert_eq!(accounts.len(), num);
    }

    #[test]
    fn test_write_recover() {
        let num = 100;
        let (client, accounts) = generate_accounts_from_wallet(num);
        assert_eq!(accounts.len(), num);

        let file = NamedTempFile::new().unwrap();
        let path = file.into_temp_path();
        io_utils::write_recovery(&client.wallet, &path).expect("failed to write to file");

        let wallet = io_utils::recover(&path).expect("failed to load from file");

        assert_eq!(client.wallet.mnemonic(), wallet.mnemonic());
    }
}
