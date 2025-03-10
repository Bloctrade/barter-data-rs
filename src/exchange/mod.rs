use self::subscription::ExchangeSub;
use crate::subscription::SubKind;
use crate::{
    subscriber::{validator::SubscriptionValidator, Subscriber},
    subscription::Map,
    MarketStream,
};
use barter_integration::{
    error::SocketError, model::Instrument, protocol::websocket::WsMessage, Validator,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    time::Duration,
};
use url::Url;

/// `BinanceSpot` & `BinanceFuturesUsd` [`Connector`] and [`StreamSelector`] implementations.
pub mod binance;

/// `BinanceSpot` & `BinanceFuturesUsd` [`Connector`] and [`StreamSelector`] implementations.
pub mod binance_paper;


/// `Bitfinex` [`Connector`] and [`StreamSelector`] implementations.
pub mod bitfinex;

/// `Coinbase` [`Connector`] and [`StreamSelector`] implementations.
pub mod coinbase;

/// `GateioSpot`, `GateioFuturesUsd` & `GateioFuturesBtc` [`Connector`] and [`StreamSelector`]
/// implementations.
pub mod gateio;

/// `Kraken` [`Connector`] and [`StreamSelector`] implementations.
pub mod kraken;

/// `Kraken` [`Connector`] and [`StreamSelector`] implementations.
pub mod kraken_paper;

/// `Okx` [`Connector`] and [`StreamSelector`] implementations.
pub mod okx;

/// Defines the generic [`ExchangeSub`] containing a market and channel combination used by an
/// exchange [`Connector`] to build [`WsMessage`] subscription payloads.
pub mod subscription;

/// Default [`Duration`] the [`Connector::SubValidator`] will wait to receive all success responses to actioned
/// [`Subscription`](crate::subscription::Subscription) requests.
pub const DEFAULT_SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Defines the [`MarketStream`] kind associated with an exchange
/// [`Subscription`](crate::subscription::Subscription) [`SubKind`](crate::subscription::SubKind).
///
/// ### Notes
/// Must be implemented by an exchange [`Connector`] if it supports a specific
/// [`SubKind`](crate::subscription::SubKind).
pub trait StreamSelector<Kind>
where
    Self: Connector,
    Kind: SubKind,
{
    type Stream: MarketStream<Self, Kind>;
}

/// Primary exchange abstraction. Defines how to translate Barter types into exchange specific
/// types, as well as connecting, subscribing, and interacting with the exchange server.
///
/// ### Notes
/// This must be implemented for a new exchange integration!
pub trait Connector
where
    Self: Clone + Default + Debug + for<'de> Deserialize<'de> + Serialize + Sized,
{
    /// Unique identifier for the exchange server being connected with.
    const ID: ExchangeId;

    /// Type that defines how to translate a Barter
    /// [`Subscription`](crate::subscription::Subscription) into an exchange specific channel
    /// to be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceChannel("@depth@100ms")`](binance::channel::BinanceChannel)
    /// - [`KrakenChannel("trade")`](kraken::channel::KrakenChannel)
    type Channel: AsRef<str>;

    /// Type that defines how to translate a Barter
    /// [`Subscription`](crate::subscription::Subscription) into an exchange specific market that
    /// can be subscribed to.
    ///
    /// ### Examples
    /// - [`BinanceMarket("btcusdt")`](binance::market::BinanceMarket)
    /// - [`KrakenMarket("BTC/USDT")`](kraken::market::KrakenMarket)
    type Market: AsRef<str>;

    /// [`Subscriber`] type that establishes a connection with the exchange server, and actions
    /// [`Subscription`](crate::subscription::Subscription)s over the socket.
    type Subscriber: Subscriber;

    /// [`SubscriptionValidator`] type that listens to responses from the exchange server and
    /// validates if the actioned [`Subscription`](crate::subscription::Subscription)s were
    /// successful.
    type SubValidator: SubscriptionValidator;

    /// Deserialisable type that the [`Self::SubValidator`] expects to receive from the exchange server in
    /// response to the [`Subscription`](crate::subscription::Subscription) [`Self::requests`]
    /// sent over the [`WebSocket`](barter_integration::protocol::websocket::WebSocket). Implements
    /// [`Validator`](barter_integration::Validator) in order to determine if [`Self`]
    /// communicates a successful [`Subscription`](crate::subscription::Subscription) outcome.
    type SubResponse: Validator + Debug + DeserializeOwned;

    /// Base [`Url`] of the exchange server being connected with.
    fn url() -> Result<Url, SocketError>;

    /// Defines [`PingInterval`] of custom application-level
    /// [`WebSocket`](barter_integration::protocol::websocket::WebSocket) pings for the exchange
    /// server being connected with.
    ///
    /// Defaults to `None`, meaning that no custom pings are sent.
    fn ping_interval() -> Option<PingInterval> {
        None
    }

    /// Defines how to translate a collection of [`ExchangeSub`]s into the [`WsMessage`]
    /// subscription payloads sent to the exchange server.
    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage>;

    /// Number of [`Subscription`](crate::subscription::Subscription) responses expected from the
    /// exchange server in responses to the requests send. Used to validate all
    /// [`Subscription`](crate::subscription::Subscription)s were accepted.
    fn expected_responses(map: &Map<Instrument>) -> usize {
        map.0.len()
    }

    /// Expected [`Duration`] the [`SubscriptionValidator`] will wait to receive all success
    /// responses to actioned [`Subscription`](crate::subscription::Subscription) requests.
    fn subscription_timeout() -> Duration {
        DEFAULT_SUBSCRIPTION_TIMEOUT
    }
}

/// Used when an exchange has servers different
/// [`InstrumentKind`](barter_integration::model::InstrumentKind) market data on distinct servers,
/// allowing all the [`Connector`] logic to be identical apart from what this trait provides.
///
/// ### Examples
/// - [`BinanceServerSpot`](binance::spot::BinanceServerSpot)
/// - [`BinanceServerFuturesUsd`](binance::futures::BinanceServerFuturesUsd)
pub trait ExchangeServer: Default + Debug + Clone + Send {
    const ID: ExchangeId;
    fn websocket_url() -> &'static str;
}

/// Defines the frequency and construction function for custom
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket) pings - used for exchanges
/// that require additional application-level pings.
#[derive(Debug)]
pub struct PingInterval {
    pub interval: tokio::time::Interval,
    pub ping: fn() -> WsMessage,
}

/// Unique identifier an exchange server [`Connector`].
///
/// ### Notes
/// An exchange may server different [`InstrumentKind`](barter_integration::model::InstrumentKind)
/// market data on distinct servers (eg/ Binance, Gateio). Such exchanges have multiple [`Self`]
/// variants, and often utilise the [`ExchangeServer`] trait.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "exchange", rename_all = "snake_case")]
pub enum ExchangeId {
    BinanceFuturesUsd,
    BinanceSpot,
    Bitfinex,
    Coinbase,
    GateioFuturesBtc,
    GateioFuturesUsd,
    GateioSpot,
    Kraken,
    Okx,
}

impl From<ExchangeId> for barter_integration::model::Exchange {
    fn from(exchange_id: ExchangeId) -> Self {
        barter_integration::model::Exchange::from(exchange_id.as_str())
    }
}

impl Display for ExchangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeId {
    /// Return the &str representation of this [`ExchangeId`]
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::BinanceSpot => "binance_spot",
            ExchangeId::BinanceFuturesUsd => "binance_futures_usd",
            ExchangeId::Bitfinex => "bitfinex",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::GateioSpot => "gateio_spot",
            ExchangeId::GateioFuturesUsd => "gateio_futures_usd",
            ExchangeId::GateioFuturesBtc => "gateio_futures_btc",
            ExchangeId::Kraken => "kraken",
            ExchangeId::Okx => "okx",
        }
    }

    /// Determines whether the [`Connector`] associated with this [`ExchangeId`] supports the
    /// ingestion of [`InstrumentKind::Spot`](barter_integration::model::InstrumentKind) market data.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_spot(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => false,
            _ => true,
        }
    }

    /// Determines whether the [`Connector`] associated with this [`ExchangeId`] supports the
    /// collection of [`InstrumentKind::Future**`](barter_integration::model::InstrumentKind)
    /// market data.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_futures(&self) -> bool {
        match self {
            ExchangeId::BinanceFuturesUsd => true,
            ExchangeId::Okx => true,
            _ => false,
        }
    }
}
