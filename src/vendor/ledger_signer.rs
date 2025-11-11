//! Ledger Ethereum app wrapper.
//! This code is modified from Alloy to allow holding `Ledger` as an Arc mutex
//! https://github.com/foundry-rs/alloy/blob/main/crates/signer-ledger/src/lib.rs

use std::fmt;
use std::sync::Arc;

use alloy::consensus::SignableTransaction;
use alloy::primitives::{hex, normalize_v, Address, ChainId, Signature, SignatureError, B256};
use alloy::signers::{ledger::HDPath as DerivationType, ledger::LedgerError, Result, Signer};
use async_trait::async_trait;
use coins_ledger::{
    common::{APDUCommand, APDUData},
    transports::{Ledger, LedgerAsync},
};
use futures_util::lock::Mutex;

macro_rules! sign_transaction_with_chain_id {
    // async (
    //    signer: impl Signer,
    //    tx: &mut impl SignableTransaction<Signature>,
    //    sign: lazy Signature,
    // )
    ($signer:expr, $tx:expr, $sign:expr) => {{
        if let Some(chain_id) = $signer.chain_id() {
            if !$tx.set_chain_id_checked(chain_id) {
                return Err(alloy::signers::Error::TransactionChainIdMismatch {
                    signer: chain_id,
                    // we can only end up here if the tx has a chain id
                    tx: $tx.chain_id().unwrap(),
                });
            }
        }

        $sign.map_err(alloy::signers::Error::other)
    }};
}

const P1_FIRST_0: u8 = 0x00;
const P1_FIRST_1: u8 = 0x01;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(non_camel_case_types)]
#[allow(dead_code)] // Some variants are only used with certain features.
#[allow(clippy::upper_case_acronyms)]
enum INS {
    GET_PUBLIC_KEY = 0x02,
    SIGN = 0x04,
    GET_APP_CONFIGURATION = 0x06,
    SIGN_PERSONAL_MESSAGE = 0x08,
    SIGN_ETH_EIP_712 = 0x0C,
    SIGN_EIP7702_AUTHORIZATION = 0x34,
}

impl fmt::Display for INS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GET_PUBLIC_KEY => write!(f, "GET_PUBLIC_KEY"),
            Self::SIGN => write!(f, "SIGN"),
            Self::GET_APP_CONFIGURATION => write!(f, "GET_APP_CONFIGURATION"),
            Self::SIGN_PERSONAL_MESSAGE => write!(f, "SIGN_PERSONAL_MESSAGE"),
            Self::SIGN_ETH_EIP_712 => write!(f, "SIGN_ETH_EIP_712"),
            Self::SIGN_EIP7702_AUTHORIZATION => write!(f, "SIGN_EIP7702_AUTHORIZATION"),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
enum P1 {
    NON_CONFIRM = 0x00,
    MORE = 0x80,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(non_camel_case_types)]
enum P2 {
    NO_CHAINCODE = 0x00,
}

// Helper to encode a big-endian varint (no leading zeroes)
// Nonce limit is 2**64 - 1 https://eips.ethereum.org/EIPS/eip-2681
fn be_varint(n: u64) -> Vec<u8> {
    let mut buf = n.to_be_bytes().to_vec();
    while buf.first() == Some(&0) && buf.len() > 1 {
        buf.remove(0);
    }
    buf
}

// Tlv encoding for the 7702 authorization list
fn make_eip7702_tlv(chain_id: alloy::primitives::U256, delegate: &[u8; 20], nonce: u64) -> Vec<u8> {
    let mut tlv = Vec::with_capacity(9 + 20);

    // STRUCT_VERSION tag=0x00, one-byte version=1
    tlv.push(0x00);
    tlv.push(1);
    tlv.push(1);

    // DELEGATE_ADDR tag=0x01
    tlv.push(0x01);
    tlv.push(20);
    tlv.extend_from_slice(delegate);

    // CHAIN_ID tag=0x02
    let ci = be_varint(chain_id.to::<u64>());
    tlv.push(0x02);
    tlv.push(ci.len() as u8);
    tlv.extend_from_slice(&ci);

    // NONCE tag=0x03
    let nn = be_varint(nonce);
    tlv.push(0x03);
    tlv.push(nn.len() as u8);
    tlv.extend_from_slice(&nn);

    tlv
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
    derivation: DerivationType,
    pub(crate) chain_id: Option<ChainId>,
    pub(crate) address: Address,
}

// Required for IntoSigner
#[cfg(target_family = "wasm")]
unsafe impl Send for LedgerSigner {}
#[cfg(target_family = "wasm")]
unsafe impl Sync for LedgerSigner {}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
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
        let encoded = tx.encoded_for_signing();

        match encoded.as_slice() {
            // Ledger requires passing EIP712 data to a separate instruction
            [0x19, 0x1, data @ ..] => {
                let domain_sep = data
                    .get(..32)
                    .ok_or_else(|| {
                        alloy::signers::Error::other(
                            "eip712 encoded data did not have a domain separator",
                        )
                    })
                    .map(B256::from_slice)?;

                let hash = data[32..]
                    .get(..32)
                    .ok_or_else(|| {
                        alloy::signers::Error::other("eip712 encoded data did not have hash struct")
                    })
                    .map(B256::from_slice)?;

                sign_transaction_with_chain_id!(
                    self,
                    tx,
                    self.sign_typed_data_with_separator(&hash, &domain_sep)
                        .await
                )
            }
            // Usual flow
            encoded => sign_transaction_with_chain_id!(self, tx, self.sign_tx_rlp(encoded).await),
        }
    }
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl Signer for LedgerSigner {
    async fn sign_hash(&self, _hash: &B256) -> Result<Signature> {
        Err(alloy::signers::Error::UnsupportedOperation(
            alloy::signers::UnsupportedSignerOperation::SignHash,
        ))
    }

    #[inline]
    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        let mut payload = Self::path_to_bytes(&self.derivation);
        // Ensure message length fits into u32 as required by Ledger APDU format.
        let msg_len_u32 = u32::try_from(message.len())
            .map_err(|_| alloy::signers::Error::other("message too long (>4GiB)"))?;
        payload.extend_from_slice(&msg_len_u32.to_be_bytes());
        payload.extend_from_slice(message);

        self.sign_payload(INS::SIGN_PERSONAL_MESSAGE, &payload)
            .await
            .map_err(alloy::signers::Error::other)
    }

    #[inline]
    async fn sign_typed_data<T: alloy::sol_types::SolStruct + Send + Sync>(
        &self,
        payload: &T,
        domain: &alloy::sol_types::Eip712Domain,
    ) -> Result<Signature> {
        self.sign_typed_data_(&payload.eip712_hash_struct(), domain)
            .await
            .map_err(alloy::signers::Error::other)
    }

    #[inline]
    async fn sign_dynamic_typed_data(
        &self,
        payload: &alloy::dyn_abi::TypedData,
    ) -> Result<Signature> {
        self.sign_typed_data_(&payload.hash_struct()?, &payload.domain)
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

alloy::network::impl_into_wallet!(LedgerSigner);

impl LedgerSigner {
    /// Instantiate the application by acquiring a lock on the ledger device.
    pub async fn new(
        transport: Arc<Mutex<Ledger>>,
        derivation: DerivationType,
        chain_id: Option<ChainId>,
    ) -> Result<Self, LedgerError> {
        let address = {
            let transport_guard = transport.lock().await;
            Self::get_address_with_path_transport(&transport_guard, &derivation).await?
        };

        Ok(Self {
            transport,
            derivation,
            chain_id,
            address,
        })
    }

    #[allow(dead_code)]
    /// Instantiate the application using a existing transport.
    pub async fn new_with_transport(
        derivation: DerivationType,
        chain_id: Option<ChainId>,
        transport: Arc<Mutex<Ledger>>,
    ) -> Result<Self, LedgerError> {
        let address = {
            let transport_guard = transport.lock().await;
            Self::get_address_with_path_transport(&transport_guard, &derivation).await?
        };

        Ok(Self {
            transport,
            derivation,
            chain_id,
            address,
        })
    }

    #[allow(dead_code)]
    /// Get the account which corresponds to our derivation path
    pub async fn get_address(&self) -> Result<Address, LedgerError> {
        self.get_address_with_path(&self.derivation).await
    }

    /// Gets the account which corresponds to the provided derivation path
    pub async fn get_address_with_path(
        &self,
        derivation: &DerivationType,
    ) -> Result<Address, LedgerError> {
        let transport = self.transport.lock().await;
        Self::get_address_with_path_transport(&transport, derivation).await
    }

    async fn get_address_with_path_transport(
        transport: &Ledger,
        derivation: &DerivationType,
    ) -> Result<Address, LedgerError> {
        let data = APDUData::new(&Self::path_to_bytes(derivation));

        let command = APDUCommand {
            cla: 0xe0,
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
    pub async fn version(&self) -> Result<semver::Version, LedgerError> {
        let transport = self.transport.lock().await;

        let command = APDUCommand {
            cla: 0xe0,
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

    async fn sign_typed_data_with_separator(
        &self,
        hash_struct: &B256,
        separator: &B256,
    ) -> Result<Signature, LedgerError> {
        // See comment for v1.6.0 requirement
        // https://github.com/LedgerHQ/app-ethereum/issues/105#issuecomment-765316999
        const EIP712_MIN_VERSION: &str = ">=1.6.0";
        let req = semver::VersionReq::parse(EIP712_MIN_VERSION)?;
        let version = self.version().await?;

        // Enforce app version is greater than EIP712_MIN_VERSION
        if !req.matches(&version) {
            return Err(LedgerError::UnsupportedAppVersion(EIP712_MIN_VERSION));
        }

        let mut data = Self::path_to_bytes(&self.derivation);
        data.extend_from_slice(separator.as_slice());
        data.extend_from_slice(hash_struct.as_slice());

        self.sign_payload(INS::SIGN_ETH_EIP_712, &data).await
    }

    async fn sign_typed_data_(
        &self,
        hash_struct: &B256,
        domain: &alloy::sol_types::Eip712Domain,
    ) -> Result<Signature, LedgerError> {
        self.sign_typed_data_with_separator(hash_struct, &domain.separator())
            .await
    }

    #[allow(dead_code)]
    /// Sign “auth data” per EIP-7702:
    /// msg = keccak256(0x05 ‖ rlp([chain_id, address, nonce]))
    pub async fn sign_auth(
        &self,
        auth: &alloy::eips::eip7702::Authorization,
    ) -> Result<Signature, LedgerError> {
        let path_bytes = Self::path_to_bytes(&self.derivation);
        let tlv_payload = make_eip7702_tlv(auth.chain_id, &auth.address, auth.nonce);

        let tlv_length = (tlv_payload.len() as u16).to_be_bytes();

        let mut payload = Vec::with_capacity(path_bytes.len() + 2 + tlv_payload.len());
        payload.extend_from_slice(&path_bytes);
        payload.extend_from_slice(&tlv_length);
        payload.extend_from_slice(&tlv_payload);

        self.sign_payload(INS::SIGN_EIP7702_AUTHORIZATION, &payload)
            .await
    }

    /// Helper function for signing either transaction data, personal messages or EIP712 derived
    /// structs.
    async fn sign_payload(&self, command: INS, payload: &[u8]) -> Result<Signature, LedgerError> {
        // @note Because tlv encoding is done on 7702 auth types sig, it checks if chunks are the
        // header or continuations. @note We need to mention the starter chunk first.
        let p1_first = if command == INS::SIGN_EIP7702_AUTHORIZATION {
            P1_FIRST_1
        } else {
            P1_FIRST_0
        };
        let transport = self.transport.lock().await;
        let mut command = APDUCommand {
            cla: 0xe0,
            ins: command as u8,
            p1: p1_first,
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
            let _data = ans.data().ok_or(LedgerError::UnexpectedNullResponse)?;
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

        let parity = normalize_v(data[0] as u64).ok_or(LedgerError::SignatureError(
            SignatureError::InvalidParity(data[0] as u64),
        ))?;
        let sig = Signature::from_bytes_and_parity(&data[1..], parity);
        Ok(sig)
    }

    // helper which converts a derivation path to bytes
    fn path_to_bytes(derivation: &DerivationType) -> Vec<u8> {
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
