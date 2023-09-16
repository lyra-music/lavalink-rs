use crate::client::LavalinkClient;
use crate::error::LavalinkError;
use crate::model::{events, BoxFuture, UserId};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use async_tungstenite::tungstenite::Message as TungsteniteMessage;
use async_tungstenite::{tokio::connect_async, tungstenite::handshake::client::generate_key};
use futures::stream::StreamExt;
use http::Request;
use reqwest::header::HeaderMap;

#[derive(Hash, Debug, Clone, Default)]
#[cfg_attr(feature = "python", pyo3::pyclass)]
/// A builder for the node.
///
/// # Example
///
/// ```
/// # use crate::model::UserId;
/// let node_builder = NodeBuilder {
///     hostname: "localhost:2333".to_string(),
///     password: "youshallnotpass".to_string(),
///     user_id: UserId(551759974905151548),
///     ..Default::default()
/// };
/// ```
pub struct NodeBuilder {
    /// The hostname of the Lavalink server.
    ///
    /// Example: "localhost:2333"
    pub hostname: String,
    /// If the Lavalink server is behind SSL encryption.
    pub is_ssl: bool,
    /// The event handler specific for this node.
    ///
    /// In most cases, the default is good.
    pub events: events::Events,
    /// The Lavalink server password.
    pub password: String,
    /// The bot User ID that will use Lavalink.
    pub user_id: UserId,
    /// The previous Session ID if resuming.
    pub session_id: Option<String>,
}

#[derive(Debug)]
#[cfg_attr(feature = "python", pyo3::pyclass)]
/// A Lavalink server node.
pub struct Node {
    pub id: usize,
    pub session_id: ArcSwap<String>,
    pub websocket_address: String,
    pub http: crate::http::Http,
    pub events: events::Events,
    pub is_running: AtomicBool,
    pub password: String,
    pub user_id: UserId,
}

#[derive(Copy, Clone)]
struct EventDispatcher<'a>(&'a Node, &'a LavalinkClient);

// Thanks Alba :D
impl<'a> EventDispatcher<'a> {
    pub(crate) async fn dispatch<T, F>(self, event: T, handler: F)
    where
        F: Fn(&events::Events) -> Option<fn(LavalinkClient, String, &T) -> BoxFuture<()>>,
    {
        let EventDispatcher(self_node, lavalink_client) = self;
        let session_id = self_node.session_id.load_full();
        let targets = [&self_node.events, &lavalink_client.events].into_iter();

        for handler in targets.filter_map(handler) {
            handler(lavalink_client.clone(), (*session_id).clone(), &event).await;
        }
    }

    pub(crate) async fn parse_and_dispatch<T, F>(self, event: &'a str, handler: F)
    where
        F: Fn(&events::Events) -> Option<fn(LavalinkClient, String, &T) -> BoxFuture<()>>,
        T: serde::Deserialize<'a>,
    {
        trace!("{:?}", event);
        let event = serde_json::from_str(event).unwrap();
        self.dispatch(event, handler).await
    }
}

impl Node {
    /// Create a connection to the Lavalink server.
    pub async fn connect(&self, lavalink_client: LavalinkClient) -> Result<(), LavalinkError> {
        let mut url = Request::builder()
            .method("GET")
            .header("Host", &self.websocket_address)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_key())
            .uri(&self.websocket_address)
            .body(())?;

        {
            let ref_headers = url.headers_mut();

            let mut headers = HeaderMap::new();
            headers.insert("Authorization", self.password.parse()?);
            headers.insert("User-Id", self.user_id.0.to_string().parse()?);
            headers.insert("Session-Id", self.session_id.to_string().parse()?);
            headers.insert(
                "Client-Name",
                format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"),)
                    .to_string()
                    .parse()?,
            );

            ref_headers.extend(headers.clone());
        }

        let (ws_stream, _) = connect_async(url).await?;

        info!("Connected to {}", self.websocket_address);

        let (_write, mut read) = ws_stream.split();

        self.is_running.store(true, Ordering::Relaxed);

        let self_node_id = self.id;

        tokio::spawn(async move {
            while let Some(Ok(resp)) = read.next().await {
                let x = match resp {
                    TungsteniteMessage::Text(x) => x,
                    _ => continue,
                };

                let base_event = match serde_json::from_str::<serde_json::Value>(&x) {
                    Ok(base_event) => base_event,
                    _ => continue,
                };

                let lavalink_client = lavalink_client.clone();

                tokio::spawn(async move {
                    let self_node = lavalink_client.nodes.get(self_node_id).unwrap();
                    let ed = EventDispatcher(self_node, &lavalink_client);

                    match base_event.get("op").unwrap().as_str().unwrap() {
                        "ready" => {
                            let ready_event: events::Ready = serde_json::from_str(&x).unwrap();

                            self_node
                                .session_id
                                .swap(Arc::new(ready_event.session_id.to_string()));

                            ed.dispatch(ready_event, |e| e.ready).await;
                        }
                        "playerUpdate" => {
                            let player_update_event: events::PlayerUpdate =
                                serde_json::from_str(&x).unwrap();

                            if let Some(player) =
                                lavalink_client.get_player_context(player_update_event.guild_id)
                            {
                                if let Err(why) = player.update_state(player_update_event.state) {
                                    error!(
                                        "Error updating state for player {}: {}",
                                        player_update_event.guild_id.0, why
                                    );
                                }
                            }

                            ed.parse_and_dispatch(&x, |e| e.player_update).await;
                        }
                        "stats" => ed.parse_and_dispatch(&x, |e| e.stats).await,
                        "event" => match base_event.get("type").unwrap().as_str().unwrap() {
                            "TrackStartEvent" => {
                                let track_event: events::TrackStart =
                                    serde_json::from_str(&x).unwrap();

                                if let Some(player) =
                                    lavalink_client.get_player_context(track_event.guild_id)
                                {
                                    if let Err(why) = player.update_track(track_event.track.into())
                                    {
                                        error!(
                                            "Error sending update track message for player {}: {}",
                                            track_event.guild_id.0, why
                                        );
                                    }
                                }

                                ed.parse_and_dispatch(&x, |e| e.track_start).await;
                            }
                            "TrackEndEvent" => {
                                let track_event: events::TrackEnd =
                                    serde_json::from_str(&x).unwrap();

                                if let Some(player) =
                                    lavalink_client.get_player_context(track_event.guild_id)
                                {
                                    if let Err(why) = player.finish(track_event.reason.into()) {
                                        error!(
                                            "Error sending finish message for player {}: {}",
                                            track_event.guild_id.0, why
                                        );
                                    }

                                    if let Err(why) = player.update_track(track_event.track.into())
                                    {
                                        error!(
                                            "Error sending update track message for player {}: {}",
                                            track_event.guild_id.0, why
                                        );
                                    }
                                }

                                ed.parse_and_dispatch(&x, |e| e.track_end).await;
                            }
                            "TrackExceptionEvent" => {
                                ed.parse_and_dispatch(&x, |e| e.track_exception).await;
                            }
                            "TrackStuckEvent" => ed.parse_and_dispatch(&x, |e| e.track_stuck).await,
                            "WebSocketClosedEvent" => {
                                ed.parse_and_dispatch(&x, |e| e.websocket_closed).await;
                            }
                            _ => (),
                        },

                        _ => (),
                    }

                    ed.dispatch(base_event, |e| e.raw).await;
                });
            }

            let self_node = lavalink_client.nodes.get(self_node_id).unwrap();
            self_node.is_running.store(false, Ordering::Relaxed);
            error!("Connection Closed.");
        });

        Ok(())
    }
}
