// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! This crate provides Protocol Buffers definitions for the services provided by the
//! [`storage_service`](../storage_service/index.html) crate.
//!
//! The protocol is documented in Protocol Buffers sources files in the `.proto` extension and the
//! documentation is not viewable via rustdoc. Refer to the source code to see it.
//!
//! The content provided in this documentation falls to two categories:
//!
//!   1. Those automatically generated by [`grpc-rs`](https://github.com/pingcap/grpc-rs):
//!       * In [`proto::storage`] are structs corresponding to our Protocol Buffers messages.
//!       * In [`proto::storage_grpc`] live the [GRPC](grpc.io) client struct and the service trait
//! which correspond to our Protocol Buffers services.
//!   1. Structs we wrote manually as helpers to ease the manipulation of the above category of
//! structs. By implementing the [`FromProto`](proto_conv::FromProto) and
//! [`IntoProto`](proto_conv::IntoProto) traits, these structs convert from/to the above category of
//! structs in a single method call and in that process data integrity check can be done. These live
//! right in the root module of this crate (this page).
//!
//! Ihis is provided as a separate crate so that crates that use the storage service via
//! [`storage_client`](../storage_client/index.html) don't need to depending on the entire
//! [`storage_service`](../storage_client/index.html).

pub mod proto;

use crypto::HashValue;
use failure::prelude::*;
#[cfg(any(test, feature = "testing"))]
use proptest_derive::Arbitrary;
use proto_conv::{FromProto, IntoProto};
use std::convert::TryFrom;
use types::{
    account_address::AccountAddress,
    account_state_blob::AccountStateBlob,
    crypto_proxies::LedgerInfoWithSignatures,
    ledger_info::LedgerInfo,
    proof::SparseMerkleProof,
    transaction::{TransactionListWithProof, TransactionToCommit, Version},
};

/// Helper to construct and parse [`proto::storage::GetAccountStateWithProofByVersionRequest`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(PartialEq, Eq, Clone, IntoProto)]
#[ProtoType(crate::proto::storage::GetAccountStateWithProofByVersionRequest)]
pub struct GetAccountStateWithProofByVersionRequest {
    /// The access path to query with.
    pub address: AccountAddress,

    /// The version the query is based on.
    pub version: Version,
}

impl GetAccountStateWithProofByVersionRequest {
    /// Constructor.
    pub fn new(address: AccountAddress, version: Version) -> Self {
        Self { address, version }
    }
}

impl FromProto for GetAccountStateWithProofByVersionRequest {
    type ProtoType = crate::proto::storage::GetAccountStateWithProofByVersionRequest;

    fn from_proto(mut object: Self::ProtoType) -> Result<Self> {
        let address = AccountAddress::from_proto(object.take_address())?;
        let version = object.get_version();
        Ok(Self { address, version })
    }
}

impl TryFrom<crate::proto::storage_prost::GetAccountStateWithProofByVersionRequest>
    for GetAccountStateWithProofByVersionRequest
{
    type Error = Error;

    fn try_from(
        proto: crate::proto::storage_prost::GetAccountStateWithProofByVersionRequest,
    ) -> Result<Self> {
        let address = AccountAddress::try_from(&proto.address[..])?;
        let version = proto.version;

        Ok(Self { address, version })
    }
}

impl From<GetAccountStateWithProofByVersionRequest>
    for crate::proto::storage_prost::GetAccountStateWithProofByVersionRequest
{
    fn from(version: GetAccountStateWithProofByVersionRequest) -> Self {
        Self {
            address: version.address.into(),
            version: version.version,
        }
    }
}

/// Helper to construct and parse [`proto::storage::GetAccountStateWithProofByVersionResponse`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(PartialEq, Eq, Clone)]
pub struct GetAccountStateWithProofByVersionResponse {
    /// The account state blob requested.
    pub account_state_blob: Option<AccountStateBlob>,

    /// The state root hash the query is based on.
    pub sparse_merkle_proof: SparseMerkleProof,
}

impl GetAccountStateWithProofByVersionResponse {
    /// Constructor.
    pub fn new(
        account_state_blob: Option<AccountStateBlob>,
        sparse_merkle_proof: SparseMerkleProof,
    ) -> Self {
        Self {
            account_state_blob,
            sparse_merkle_proof,
        }
    }
}

impl FromProto for GetAccountStateWithProofByVersionResponse {
    type ProtoType = crate::proto::storage::GetAccountStateWithProofByVersionResponse;

    fn from_proto(mut object: Self::ProtoType) -> Result<Self> {
        let account_state_blob = if object.has_account_state_blob() {
            Some(AccountStateBlob::from_proto(
                object.take_account_state_blob(),
            )?)
        } else {
            None
        };
        Ok(Self {
            account_state_blob,
            sparse_merkle_proof: SparseMerkleProof::from_proto(object.take_sparse_merkle_proof())?,
        })
    }
}

impl IntoProto for GetAccountStateWithProofByVersionResponse {
    type ProtoType = crate::proto::storage::GetAccountStateWithProofByVersionResponse;

    fn into_proto(self) -> Self::ProtoType {
        let mut object = Self::ProtoType::new();

        if let Some(account_state_blob) = self.account_state_blob {
            object.set_account_state_blob(account_state_blob.into_proto());
        }
        object.set_sparse_merkle_proof(self.sparse_merkle_proof.into_proto());
        object
    }
}

impl TryFrom<crate::proto::storage_prost::GetAccountStateWithProofByVersionResponse>
    for GetAccountStateWithProofByVersionResponse
{
    type Error = Error;

    fn try_from(
        proto: crate::proto::storage_prost::GetAccountStateWithProofByVersionResponse,
    ) -> Result<Self> {
        let account_state_blob = proto
            .account_state_blob
            .map(AccountStateBlob::try_from)
            .transpose()?;
        Ok(Self {
            account_state_blob,
            sparse_merkle_proof: SparseMerkleProof::try_from(
                proto.sparse_merkle_proof.unwrap_or_else(Default::default),
            )?,
        })
    }
}

impl From<GetAccountStateWithProofByVersionResponse>
    for crate::proto::storage_prost::GetAccountStateWithProofByVersionResponse
{
    fn from(response: GetAccountStateWithProofByVersionResponse) -> Self {
        Self {
            account_state_blob: response.account_state_blob.map(Into::into),
            sparse_merkle_proof: Some(response.sparse_merkle_proof.into()),
        }
    }
}

impl Into<(Option<AccountStateBlob>, SparseMerkleProof)>
    for GetAccountStateWithProofByVersionResponse
{
    fn into(self) -> (Option<AccountStateBlob>, SparseMerkleProof) {
        (self.account_state_blob, self.sparse_merkle_proof)
    }
}

/// Helper to construct and parse [`proto::storage::SaveTransactionsRequest`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Arbitrary))]
pub struct SaveTransactionsRequest {
    pub txns_to_commit: Vec<TransactionToCommit>,
    pub first_version: Version,
    pub ledger_info_with_signatures: Option<LedgerInfoWithSignatures>,
}

impl SaveTransactionsRequest {
    /// Constructor.
    pub fn new(
        txns_to_commit: Vec<TransactionToCommit>,
        first_version: Version,
        ledger_info_with_signatures: Option<LedgerInfoWithSignatures>,
    ) -> Self {
        SaveTransactionsRequest {
            txns_to_commit,
            first_version,
            ledger_info_with_signatures,
        }
    }
}

impl FromProto for SaveTransactionsRequest {
    type ProtoType = crate::proto::storage::SaveTransactionsRequest;

    fn from_proto(mut object: Self::ProtoType) -> Result<Self> {
        let txns_to_commit = object
            .take_txns_to_commit()
            .into_iter()
            .map(TransactionToCommit::from_proto)
            .collect::<Result<Vec<_>>>()?;
        let first_version = object.get_first_version();
        let ledger_info_with_signatures = object
            .ledger_info_with_signatures
            .take()
            .map(LedgerInfoWithSignatures::from_proto)
            .transpose()?;

        Ok(Self {
            txns_to_commit,
            first_version,
            ledger_info_with_signatures,
        })
    }
}

impl IntoProto for SaveTransactionsRequest {
    type ProtoType = crate::proto::storage::SaveTransactionsRequest;

    fn into_proto(self) -> Self::ProtoType {
        let mut proto = Self::ProtoType::new();
        proto.set_txns_to_commit(::protobuf::RepeatedField::from_vec(
            self.txns_to_commit
                .into_iter()
                .map(TransactionToCommit::into_proto)
                .collect::<Vec<_>>(),
        ));
        proto.set_first_version(self.first_version);
        if let Some(x) = self.ledger_info_with_signatures {
            proto.set_ledger_info_with_signatures(x.into_proto())
        }

        proto
    }
}

/// Helper to construct and parse [`proto::storage::GetTransactionsRequest`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Arbitrary))]
pub struct GetTransactionsRequest {
    pub start_version: Version,
    pub batch_size: u64,
    pub ledger_version: Version,
    pub fetch_events: bool,
}

impl GetTransactionsRequest {
    /// Constructor.
    pub fn new(
        start_version: Version,
        batch_size: u64,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Self {
        GetTransactionsRequest {
            start_version,
            batch_size,
            ledger_version,
            fetch_events,
        }
    }
}

impl FromProto for GetTransactionsRequest {
    type ProtoType = crate::proto::storage::GetTransactionsRequest;

    fn from_proto(object: Self::ProtoType) -> Result<Self> {
        Ok(GetTransactionsRequest {
            start_version: object.get_start_version(),
            batch_size: object.get_batch_size(),
            ledger_version: object.get_ledger_version(),
            fetch_events: object.get_fetch_events(),
        })
    }
}

impl IntoProto for GetTransactionsRequest {
    type ProtoType = crate::proto::storage::GetTransactionsRequest;

    fn into_proto(self) -> Self::ProtoType {
        let mut out = Self::ProtoType::new();
        out.set_start_version(self.start_version);
        out.set_batch_size(self.batch_size);
        out.set_ledger_version(self.ledger_version);
        out.set_fetch_events(self.fetch_events);
        out
    }
}

/// Helper to construct and parse [`proto::storage::GetTransactionsResponse`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(Clone, Debug, Eq, PartialEq, FromProto, IntoProto)]
#[cfg_attr(any(test, feature = "testing"), derive(Arbitrary))]
#[ProtoType(crate::proto::storage::GetTransactionsResponse)]
pub struct GetTransactionsResponse {
    pub txn_list_with_proof: TransactionListWithProof,
}

impl GetTransactionsResponse {
    /// Constructor.
    pub fn new(txn_list_with_proof: TransactionListWithProof) -> Self {
        GetTransactionsResponse {
            txn_list_with_proof,
        }
    }
}

/// Helper to construct and parse [`proto::storage::StartupInfo`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Arbitrary))]
pub struct StartupInfo {
    pub ledger_info: LedgerInfo,
    pub latest_version: Version,
    pub account_state_root_hash: HashValue,
    pub ledger_frozen_subtree_hashes: Vec<HashValue>,
}

impl FromProto for StartupInfo {
    type ProtoType = crate::proto::storage::StartupInfo;

    fn from_proto(mut object: Self::ProtoType) -> Result<Self> {
        let ledger_info = LedgerInfo::from_proto(object.take_ledger_info())?;
        let latest_version = object.get_latest_version();
        let account_state_root_hash = HashValue::from_proto(object.take_account_state_root_hash())?;
        let ledger_frozen_subtree_hashes = object
            .take_ledger_frozen_subtree_hashes()
            .into_iter()
            .map(HashValue::from_proto)
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            ledger_info,
            latest_version,
            account_state_root_hash,
            ledger_frozen_subtree_hashes,
        })
    }
}

impl IntoProto for StartupInfo {
    type ProtoType = crate::proto::storage::StartupInfo;

    fn into_proto(self) -> Self::ProtoType {
        let mut proto = Self::ProtoType::new();
        proto.set_ledger_info(self.ledger_info.into_proto());
        proto.set_latest_version(self.latest_version);
        proto.set_account_state_root_hash(self.account_state_root_hash.into_proto());
        proto.set_ledger_frozen_subtree_hashes(protobuf::RepeatedField::from_vec(
            self.ledger_frozen_subtree_hashes
                .into_iter()
                .map(HashValue::into_proto)
                .collect::<Vec<_>>(),
        ));
        proto
    }
}

/// Helper to construct and parse [`proto::storage::GetStartupInfoResponse`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Arbitrary))]
pub struct GetStartupInfoResponse {
    pub info: Option<StartupInfo>,
}

impl FromProto for GetStartupInfoResponse {
    type ProtoType = crate::proto::storage::GetStartupInfoResponse;

    fn from_proto(mut object: Self::ProtoType) -> Result<Self> {
        let info = if object.has_info() {
            Some(StartupInfo::from_proto(object.take_info())?)
        } else {
            None
        };

        Ok(Self { info })
    }
}

impl IntoProto for GetStartupInfoResponse {
    type ProtoType = crate::proto::storage::GetStartupInfoResponse;

    fn into_proto(self) -> Self::ProtoType {
        let mut proto = Self::ProtoType::new();
        if let Some(info) = self.info {
            proto.set_info(info.into_proto())
        }
        proto
    }
}

/// Helper to construct and parse [`proto::storage::GetLatestLedgerInfosPerEpochRequest`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(Clone, Debug, Eq, PartialEq, FromProto, IntoProto)]
#[cfg_attr(any(test, feature = "testing"), derive(Arbitrary))]
#[ProtoType(crate::proto::storage::GetLatestLedgerInfosPerEpochRequest)]
pub struct GetLatestLedgerInfosPerEpochRequest {
    pub start_epoch: u64,
}

impl GetLatestLedgerInfosPerEpochRequest {
    /// Constructor.
    pub fn new(start_epoch: u64) -> Self {
        Self { start_epoch }
    }
}

/// Helper to construct and parse [`proto::storage::GetLatestLedgerInfosPerEpochResponse`]
///
/// It does so by implementing [`IntoProto`](#impl-IntoProto) and [`FromProto`](#impl-FromProto),
/// providing [`into_proto`](IntoProto::into_proto) and [`from_proto`](FromProto::from_proto).
#[derive(Clone, Debug, Eq, PartialEq, FromProto, IntoProto)]
#[cfg_attr(any(test, feature = "testing"), derive(Arbitrary))]
#[ProtoType(crate::proto::storage::GetLatestLedgerInfosPerEpochResponse)]
pub struct GetLatestLedgerInfosPerEpochResponse {
    pub latest_ledger_infos: Vec<LedgerInfoWithSignatures>,
}

impl GetLatestLedgerInfosPerEpochResponse {
    /// Constructor.
    pub fn new(latest_ledger_infos: Vec<LedgerInfoWithSignatures>) -> Self {
        Self {
            latest_ledger_infos,
        }
    }
}

impl Into<Vec<LedgerInfoWithSignatures>> for GetLatestLedgerInfosPerEpochResponse {
    fn into(self) -> Vec<LedgerInfoWithSignatures> {
        self.latest_ledger_infos
    }
}

pub mod prelude {
    pub use super::*;
}

#[cfg(test)]
mod tests;
