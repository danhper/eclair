use alloy::network::{Network, NetworkWallet, TransactionBuilder};
use alloy::providers::fillers::{FillerControlFlow, TxFiller};
use alloy::providers::{Provider, SendableTx, WalletProvider};
use alloy::transports::{RpcError, TransportResult};

/// A layer that signs transactions locally.
///
/// The layer uses a [`NetworkWallet`] to sign transactions sent using
/// [`Provider::send_transaction`] locally before passing them to the node with
/// [`Provider::send_raw_transaction`].
/// If no wallet is set, it is a no-op.
///
/// ```
#[derive(Clone, Debug)]
pub struct OptionalWalletFiller<W> {
    wallet: Option<W>,
}

impl<W> AsRef<Option<W>> for OptionalWalletFiller<W> {
    fn as_ref(&self) -> &Option<W> {
        &self.wallet
    }
}

impl<W> AsMut<Option<W>> for OptionalWalletFiller<W> {
    fn as_mut(&mut self) -> &mut Option<W> {
        &mut self.wallet
    }
}

impl<W> OptionalWalletFiller<W> {
    /// Creates a new optional wallet layer.
    pub const fn new() -> Self {
        Self { wallet: None }
    }

    pub fn set_wallet(&mut self, wallet: W) {
        self.wallet = Some(wallet);
    }

    pub fn unsafe_as_ref(&self) -> &W {
        self.wallet.as_ref().unwrap()
    }

    pub fn unsafe_as_mut(&mut self) -> &mut W {
        self.wallet.as_mut().unwrap()
    }
}

impl<W> Default for OptionalWalletFiller<W> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W, N> TxFiller<N> for OptionalWalletFiller<W>
where
    N: Network,
    W: NetworkWallet<N> + Clone,
{
    type Fillable = ();

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        if tx.from().is_none() {
            return FillerControlFlow::Ready;
        }
        if self.wallet.is_none() {
            return FillerControlFlow::Finished;
        }

        match tx.complete_preferred() {
            Ok(_) => FillerControlFlow::Ready,
            Err(e) => FillerControlFlow::Missing(vec![("Wallet", e)]),
        }
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        if let Some(builder) = tx.as_mut_builder() {
            if builder.from().is_none() && self.wallet.is_some() {
                builder.set_from(self.wallet.as_ref().unwrap().default_signer_address());
            }
        }
    }

    async fn prepare<P>(
        &self,
        _provider: &P,
        _tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        Ok(())
    }

    async fn fill(
        &self,
        _fillable: Self::Fillable,
        tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if self.wallet.is_none() {
            return Ok(tx);
        }
        let builder = match tx {
            SendableTx::Builder(builder) => builder,
            _ => return Ok(tx),
        };

        let envelope = builder
            .build(self.wallet.as_ref().unwrap())
            .await
            .map_err(RpcError::local_usage)?;

        Ok(SendableTx::Envelope(envelope))
    }
}

impl<W, N> WalletProvider<N> for OptionalWalletFiller<W>
where
    W: NetworkWallet<N> + Clone,
    N: Network,
{
    type Wallet = W;

    #[inline(always)]
    fn wallet(&self) -> &Self::Wallet {
        self.unsafe_as_ref()
    }

    #[inline(always)]
    fn wallet_mut(&mut self) -> &mut Self::Wallet {
        self.unsafe_as_mut()
    }
}
