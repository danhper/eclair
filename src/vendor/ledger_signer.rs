//! Ledger Ethereum app wrapper.
//! This code is modified from Alloy to allow holding `Ledger` as an Arc mutex
//! https://github.com/alloy-rs/alloy/blob/78807cbd0e659873c32d373af963031f71bde8a2/crates/signer-ledger/src/signer.rs

use std::borrow::Borrow;
use std::fmt;
use std::sync::Arc;

use alloy::consensus::SignableTransaction;
use alloy::primitives::{hex, Address, ChainId, B256};
use alloy::signers::ledger::{HDPath, LedgerError};
use alloy::signers::{Result, Signature, Signer};
use async_trait::async_trait;
use coins_ledger::{
    common::{APDUCommand, APDUData},
    transports::{Ledger, LedgerAsync},
};
use futures_util::lock::Mutex;

pub(crate) const P1_FIRST: u8 = 0x00;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types, dead_code, clippy::upper_case_acronyms)]
pub(crate) enum INS {
    GET_PUBLIC_KEY = 0x02,
    SIGN = 0x04,
    GET_APP_CONFIGURATION = 0x06,
    SIGN_PERSONAL_MESSAGE = 0x08,
    SIGN_ETH_EIP_712 = 0x0C,
}

impl fmt::Display for INS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GET_PUBLIC_KEY => write!(f, "GET_PUBLIC_KEY"),
            Self::SIGN => write!(f, "SIGN"),
            Self::GET_APP_CONFIGURATION => write!(f, "GET_APP_CONFIGURATION"),
            Self::SIGN_PERSONAL_MESSAGE => write!(f, "SIGN_PERSONAL_MESSAGE"),
            Self::SIGN_ETH_EIP_712 => write!(f, "SIGN_ETH_EIP_712"),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub(crate) enum P1 {
    NON_CONFIRM = 0x00,
    MORE = 0x80,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub(crate) enum P2 {
    NO_CHAINCODE = 0x00,
}

/// A Ledger Ethereum signer.
///
/// This is a simple wrapper around the [Ledger transport](Ledger).
///
/// Note that this wallet only supports asynchronous operations. Calling a non-asynchronous method
/// will always return an error.
#[derive(Debug)]
pub struct LedgerSigner {
    transport: Arc<Mutex<Ledger>>,
    derivation: HDPath,
    pub(crate) chain_id: Option<ChainId>,
    pub(crate) address: Address,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl alloy::network::TxSigner<Signature> for LedgerSigner {
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    #[doc(alias = "sign_tx")]
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        if let Some(chain_id) = self.chain_id() {
            if !tx.set_chain_id_checked(chain_id) {
                return Err(alloy::signers::Error::TransactionChainIdMismatch {
                    signer: chain_id,
                    // we can only end up here if the tx has a chain id
                    tx: tx.chain_id().unwrap(),
                });
            }
        }

        let mut sig = self
            .sign_tx_rlp(&tx.encoded_for_signing())
            .await
            .map_err(alloy::signers::Error::other)?;

        if tx.use_eip155() {
            if let Some(chain_id) = self.chain_id().or_else(|| tx.chain_id()) {
                sig = sig.with_chain_id(chain_id);
            }
        }

        Ok(sig)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for LedgerSigner {
    async fn sign_hash(&self, _hash: &B256) -> Result<Signature> {
        Err(alloy::signers::Error::UnsupportedOperation(
            alloy::signers::UnsupportedSignerOperation::SignHash,
        ))
    }

    #[inline]
    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        let mut payload = Self::path_to_bytes(&self.derivation);
        payload.extend_from_slice(&(message.len() as u32).to_be_bytes());
        payload.extend_from_slice(message);

        self.sign_payload(INS::SIGN_PERSONAL_MESSAGE, &payload)
            .await
            .map_err(alloy::signers::Error::other)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    #[inline]
    fn set_chain_id(&mut self, chain_id: Option<ChainId>) {
        self.chain_id = chain_id;
    }
}

impl LedgerSigner {
    /// Instantiate the application by acquiring a lock on the ledger device.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn foo() -> Result<(), Box<dyn std::error::Error>> {
    /// use alloy_signer_ledger::{HDPath, LedgerSigner};
    ///
    /// let ledger = LedgerSigner::new(HDPath::LedgerLive(0), Some(1)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        transport: Arc<Mutex<Ledger>>,
        derivation: HDPath,
        chain_id: Option<ChainId>,
    ) -> Result<Self, LedgerError> {
        let address =
            Self::get_address_with_path_transport(transport.lock().await.borrow(), &derivation)
                .await?;

        Ok(Self {
            transport,
            derivation,
            chain_id,
            address,
        })
    }

    /// Get the account which corresponds to our derivation path
    pub async fn _get_address(&self) -> Result<Address, LedgerError> {
        self.get_address_with_path(&self.derivation).await
    }

    /// Gets the account which corresponds to the provided derivation path
    pub async fn get_address_with_path(&self, derivation: &HDPath) -> Result<Address, LedgerError> {
        let transport = self.transport.lock().await;
        Self::get_address_with_path_transport(&transport, derivation).await
    }

    async fn get_address_with_path_transport(
        transport: &Ledger,
        derivation: &HDPath,
    ) -> Result<Address, LedgerError> {
        let data = APDUData::new(&Self::path_to_bytes(derivation));

        let command = APDUCommand {
            ins: INS::GET_PUBLIC_KEY as u8,
            p1: P1::NON_CONFIRM as u8,
            p2: P2::NO_CHAINCODE as u8,
            data,
            response_len: None,
        };

        let answer = transport.exchange(&command).await?;
        let result = answer.data().ok_or(LedgerError::UnexpectedNullResponse)?;

        let address = {
            // extract the address from the response
            let offset = 1 + result[0] as usize;
            let address_str = &result[offset + 1..offset + 1 + result[offset] as usize];
            let mut address = [0; 20];
            address.copy_from_slice(&hex::decode(address_str)?);
            address.into()
        };
        Ok(address)
    }

    /// Returns the semver of the Ethereum ledger app
    pub async fn _version(&self) -> Result<semver::Version, LedgerError> {
        let transport = self.transport.lock().await;

        let command = APDUCommand {
            ins: INS::GET_APP_CONFIGURATION as u8,
            p1: P1::NON_CONFIRM as u8,
            p2: P2::NO_CHAINCODE as u8,
            data: APDUData::new(&[]),
            response_len: None,
        };

        let answer = transport.exchange(&command).await?;
        let data = answer.data().ok_or(LedgerError::UnexpectedNullResponse)?;
        let &[_flags, major, minor, patch] = data else {
            return Err(LedgerError::ShortResponse {
                got: data.len(),
                expected: 4,
            });
        };
        let version = semver::Version::new(major as u64, minor as u64, patch as u64);
        Ok(version)
    }

    /// Signs an Ethereum transaction's RLP bytes (requires confirmation on the ledger).
    ///
    /// Note that this does not apply EIP-155.
    #[doc(alias = "sign_transaction_rlp")]
    pub async fn sign_tx_rlp(&self, tx_rlp: &[u8]) -> Result<Signature, LedgerError> {
        let mut payload = Self::path_to_bytes(&self.derivation);
        payload.extend_from_slice(tx_rlp);
        self.sign_payload(INS::SIGN, &payload).await
    }

    /// Helper function for signing either transaction data, personal messages or EIP712 derived
    /// structs.
    async fn sign_payload(&self, command: INS, payload: &[u8]) -> Result<Signature, LedgerError> {
        let transport = self.transport.lock().await;
        let mut command = APDUCommand {
            ins: command as u8,
            p1: P1_FIRST,
            p2: P2::NO_CHAINCODE as u8,
            data: APDUData::new(&[]),
            response_len: None,
        };

        let mut answer = None;
        // workaround for https://github.com/LedgerHQ/app-ethereum/issues/409
        // TODO: remove in future version
        let chunk_size = (0..=255)
            .rev()
            .find(|i| payload.len() % i != 3)
            .expect("true for any length");

        // Iterate in 255 byte chunks
        for chunk in payload.chunks(chunk_size) {
            command.data = APDUData::new(chunk);

            let res = transport.exchange(&command).await;
            let ans = res?;
            answer = Some(ans);

            // We need more data
            command.p1 = P1::MORE as u8;
        }
        drop(transport);

        let answer = answer.unwrap();
        let data = answer.data().unwrap();
        if data.len() != 65 {
            return Err(LedgerError::ShortResponse {
                got: data.len(),
                expected: 65,
            });
        }

        let sig = Signature::from_bytes_and_parity(&data[1..], data[0] as u64)?;
        Ok(sig)
    }

    // helper which converts a derivation path to bytes
    fn path_to_bytes(derivation: &HDPath) -> Vec<u8> {
        let derivation = derivation.to_string();
        let elements = derivation.split('/').skip(1).collect::<Vec<_>>();
        let depth = elements.len();

        let mut bytes = vec![depth as u8];
        for derivation_index in elements {
            let hardened = derivation_index.contains('\'');
            let mut index = derivation_index.replace('\'', "").parse::<u32>().unwrap();
            if hardened {
                index |= 0x80000000;
            }

            bytes.extend(index.to_be_bytes());
        }

        bytes
    }
}
