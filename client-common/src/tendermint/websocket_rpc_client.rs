#![cfg(feature = "websocket-rpc")]
mod types;
mod websocket_rpc_loop;

pub use types::ConnectionState;

use std::collections::HashMap;
use std::iter;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use serde_json::{json, Value};
use websocket::sender::Writer;
use websocket::stream::sync::TcpStream;
use websocket::OwnedMessage;

use self::types::*;
use crate::tendermint::types::*;
use crate::tendermint::Client;
use crate::{Error, ErrorKind, Result, ResultExt};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

const WAIT_FOR_CONNECTION_SLEEP_INTERVAL: Duration = Duration::from_millis(200);
const WAIT_FOR_CONNECTION_COUNT: usize = 50;

/// Tendermint RPC Client (uses websocket in transport layer)
#[derive(Clone)]
pub struct WebsocketRpcClient {
    connection_state: Arc<Mutex<ConnectionState>>,
    websocket_writer: Arc<Mutex<Writer<TcpStream>>>,
    channel_map: Arc<Mutex<HashMap<String, SyncSender<JsonRpcResponse>>>>,
}

impl WebsocketRpcClient {
    /// Creates a new instance of `WebsocketRpcClient`
    //
    // # How it works
    //
    // - Spawns `websocket_rpc_loop`.
    // - Spawns `websocket_rpc_loop` monitor.
    pub fn new(url: &str) -> Result<Self> {
        let channel_map: Arc<Mutex<HashMap<String, SyncSender<JsonRpcResponse>>>> =
            Default::default();

        let (websocket_reader, websocket_writer) = websocket_rpc_loop::new_connection(url)?;
        let websocket_writer = Arc::new(Mutex::new(websocket_writer));

        let loop_handle = websocket_rpc_loop::spawn(
            channel_map.clone(),
            websocket_reader,
            websocket_writer.clone(),
        );

        let connection_state = websocket_rpc_loop::monitor(
            url.to_owned(),
            channel_map.clone(),
            loop_handle,
            websocket_writer.clone(),
        );

        Ok(Self {
            connection_state,
            websocket_writer,
            channel_map,
        })
    }

    /// Returns current connection state of websocket connection
    pub fn connection_state(&self) -> ConnectionState {
        *self
            .connection_state
            .lock()
            .expect("Unable to acquire lock on connection state")
    }

    /// Sends a RPC request
    //
    // # How it works
    //
    // - Prepares JSON-RPC request from `method` and `params` (generates a random `request_id`).
    // - Creates a `sync_channel` pair
    // - Inserts `channel_sender` to `channel_map` corresponding to generated `request_id`.
    // - Ensure that the `websocket_rpc_loop` is in `Connected` state.
    // - Send request websocket message.
    // - Receive response on `channel_receiver`.
    fn request(&self, method: &str, params: &[Value]) -> Result<Value> {
        let (id, channel_receiver) = self.send_request(method, params)?;
        self.receive_response(method, params, &id, channel_receiver)
    }

    /// Sends RPC requests for a batch.
    ///
    /// # Note
    ///
    /// This does not use batch JSON-RPC requests but makes multiple single JSON-RPC requests in parallel.
    fn request_batch(&self, batch_params: Vec<(&str, Vec<Value>)>) -> Result<Vec<Value>> {
        let mut receivers = Vec::with_capacity(batch_params.len());

        for (method, params) in batch_params.iter() {
            let (id, channel_receiver) = self.send_request(method, &params)?;
            receivers.push((id, channel_receiver));
        }

        receivers
            .into_iter()
            .zip(batch_params.into_iter())
            .map(|((id, channel_receiver), (method, params))| {
                self.receive_response(method, &params, &id, channel_receiver)
            })
            .collect()
    }

    /// Sends a JSON-RPC request and returns `request_id` and `response_channel`
    fn send_request(
        &self,
        method: &str,
        params: &[Value],
    ) -> Result<(String, Receiver<JsonRpcResponse>)> {
        let (message, id) = prepare_message(method, params)?;
        let (channel_sender, channel_receiver) = sync_channel::<JsonRpcResponse>(1);

        self.channel_map
            .lock()
            .expect("Unable to acquire lock on websocket request map: Lock is poisoned")
            .insert(id.clone(), channel_sender);

        self.ensure_connected()?;

        self.websocket_writer
            .lock()
            .expect("Unable to acquire lock on websocket writer: Lock is poisoned")
            .send_message(&message)
            .chain(|| {
                (
                    ErrorKind::InternalError,
                    "Unable to send message to websocket writer",
                )
            })
            .map_err(|err| {
                self.channel_map
                    .lock()
                    .expect("Unable to acquire lock on websocket request map: Lock is poisoned")
                    .remove(&id);
                err
            })?;

        Ok((id, channel_receiver))
    }

    /// Receives response from websocket for given id.
    fn receive_response(
        &self,
        method: &str,
        params: &[Value],
        id: &str,
        receiver: Receiver<JsonRpcResponse>,
    ) -> Result<Value> {
        let response = receiver
            .recv_timeout(RESPONSE_TIMEOUT)
            .chain(|| {
                (
                    ErrorKind::InternalError,
                    "Unable to receive message from channel receiver",
                )
            })
            .map_err(|err| {
                self.channel_map
                    .lock()
                    .expect("Unable to acquire lock on websocket request map: Lock is poisoned")
                    .remove(id);
                err
            })?;

        if let Some(err) = response.error {
            Err(Error::new_with_source(
                ErrorKind::TendermintRpcError,
                format!(
                    "Error response from tendermint RPC for request method ({}) and params ({:?})",
                    method, params
                ),
                Box::new(err),
            ))
        } else {
            Ok(response.result.unwrap_or_default())
        }
    }

    /// Ensures that the websocket is connected.
    fn ensure_connected(&self) -> Result<()> {
        for _ in 0..WAIT_FOR_CONNECTION_COUNT {
            if ConnectionState::Connected
                == *self
                    .connection_state
                    .lock()
                    .expect("Unable to acquire lock on connection state")
            {
                return Ok(());
            }

            thread::sleep(WAIT_FOR_CONNECTION_SLEEP_INTERVAL);
        }

        Err(Error::new(
            ErrorKind::InternalError,
            "Websocket connection disconnected",
        ))
    }

    /// Makes an RPC call and deserializes response
    fn call<T>(&self, method: &str, params: &[Value]) -> Result<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        let response_value = self.request(method, params)?;
        serde_json::from_value(response_value).chain(|| {
            (
                ErrorKind::DeserializationError,
                format!(
                    "Unable to deserialize `{}` from JSON-RPC response for params: {:?}",
                    method, params
                ),
            )
        })
    }

    /// Makes RPC call in batch and deserializes responses
    fn call_batch<T>(&self, params: Vec<(&str, Vec<Value>)>) -> Result<Vec<T>>
    where
        for<'de> T: Deserialize<'de>,
    {
        if params.is_empty() {
            // Do not send empty batch requests
            return Ok(Default::default());
        }

        if params.len() == 1 {
            // Do not send batch request when there is only one set of params
            self.call::<T>(params[0].0, &params[0].1)
                .map(|value| vec![value])
        } else {
            let response_values = self.request_batch(params.clone())?;

            response_values
                .into_iter()
                .zip(params.into_iter())
                .map(|(response_value, (method, params))| {
                    serde_json::from_value(response_value).chain(|| {
                        (
                            ErrorKind::DeserializationError,
                            format!(
                                "Unable to deserialize `{}` from JSON-RPC response for params: {:?}",
                                method, params
                            ),
                        )
                    })
                })
                .collect()
        }
    }
}

impl Client for WebsocketRpcClient {
    #[inline]
    fn genesis(&self) -> Result<Genesis> {
        self.call("genesis", Default::default())
    }

    #[inline]
    fn status(&self) -> Result<Status> {
        self.call("status", Default::default())
    }

    #[inline]
    fn block(&self, height: u64) -> Result<Block> {
        let params = [json!(height.to_string())];
        self.call("block", &params)
    }

    #[inline]
    fn block_batch<'a, T: Iterator<Item = &'a u64>>(&self, heights: T) -> Result<Vec<Block>> {
        let params = heights
            .map(|height| ("block", vec![json!(height.to_string())]))
            .collect::<Vec<(&str, Vec<Value>)>>();
        self.call_batch::<Block>(params)
    }

    #[inline]
    fn block_results(&self, height: u64) -> Result<BlockResults> {
        let params = [json!(height.to_string())];
        self.call("block_results", &params)
    }

    #[inline]
    fn block_results_batch<'a, T: Iterator<Item = &'a u64>>(
        &self,
        heights: T,
    ) -> Result<Vec<BlockResults>> {
        let params = heights
            .map(|height| ("block_results", vec![json!(height.to_string())]))
            .collect::<Vec<(&str, Vec<Value>)>>();
        self.call_batch::<BlockResults>(params)
    }

    fn broadcast_transaction(&self, transaction: &[u8]) -> Result<BroadcastTxResult> {
        let params = [json!(transaction)];
        let broadcast_tx_result: BroadcastTxResult = self.call("broadcast_tx_sync", &params)?;

        if broadcast_tx_result.code != 0 {
            Err(Error::new(
                ErrorKind::TendermintRpcError,
                broadcast_tx_result.log,
            ))
        } else {
            Ok(broadcast_tx_result)
        }
    }

    fn query(&self, path: &str, data: &[u8]) -> Result<QueryResult> {
        let params = [
            json!(path),
            json!(hex::encode(data)),
            json!(null),
            json!(null),
        ];
        let result: QueryResult = self.call("abci_query", &params)?;

        if result.code() != 0 {
            return Err(Error::new(ErrorKind::TendermintRpcError, result.log()));
        }

        Ok(result)
    }
}

fn prepare_message(method: &str, params: &[Value]) -> Result<(OwnedMessage, String)> {
    let mut rng = thread_rng();

    let id: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(7)
        .collect();

    let request = JsonRpcRequest {
        id: &id,
        jsonrpc: "2.0",
        method,
        params,
    };

    let request_json = serde_json::to_string(&request).chain(|| {
        (
            ErrorKind::SerializationError,
            "Unable to serialize RPC request to json",
        )
    })?;

    let message = OwnedMessage::Text(request_json);

    Ok((message, id))
}
