#![cfg(feature = "websocket-rpc")]
use std::collections::HashMap;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use websocket::receiver::Reader;
use websocket::sender::Writer;
use websocket::stream::sync::TcpStream;
use websocket::{ClientBuilder, OwnedMessage};

use crate::{ErrorKind, Result, ResultExt};

use super::{ConnectionState, JsonRpcResponse};

const MONITOR_RETRY_INTERVAL: Duration = Duration::from_secs(2);

/// Creates a new websocket connection with given url
pub fn new_connection(url: &str) -> Result<(Reader<TcpStream>, Writer<TcpStream>)> {
    ClientBuilder::new(url)
        .chain(|| (ErrorKind::InvalidInput, format!("Malformed url: {}", url)))?
        .connect_insecure()
        .chain(|| {
            (
                ErrorKind::InitializationError,
                format!("Unable to connect to websocket RPC at: {}", url),
            )
        })?
        .split()
        .chain(|| {
            (
                ErrorKind::InternalError,
                "Unable to split websocket reader and writer",
            )
        })
}

/// Spawns websocket rpc loop in a new thread
///
/// # How it works
///
/// - Connects to websocket server at given `url` and splits the connection in `reader` and `writer`.
/// - Spawns a thread and runs `websocket_rpc_loop` in the thread which continues until the thread panics.
/// - For each websocket message received:
///   - Parse the message into JSON-RPC response.
///   - Pop the response channel from `channel_map` corresponding to response's `request_id`.
///   - Send the response to the channel.
pub fn spawn(
    channel_map: Arc<Mutex<HashMap<String, SyncSender<JsonRpcResponse>>>>,
    mut websocket_reader: Reader<TcpStream>,
    websocket_writer: Arc<Mutex<Writer<TcpStream>>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        for message in websocket_reader.incoming_messages() {
            match message {
                Ok(message) => match message {
                    OwnedMessage::Text(ref message) => handle_text(message, channel_map.clone()),
                    OwnedMessage::Binary(ref message) => handle_slice(message, channel_map.clone()),
                    OwnedMessage::Ping(data) => send_pong(websocket_writer.clone(), data),
                    _ => {
                        log::trace!("Received unknown message: {:?}", message);
                    }
                },
                Err(err) => {
                    log::error!("Websocket error message: {}", err);
                    break;
                }
            }
        }
    })
}

/// Monitors websocket connection and retries if websocket is disconnected
///
/// # How it works
///
/// - Websocket connection has two possible states:
///   - `Connected`: `websocket_rpc_loop` is connected to websocket server
///   - `Disconnected`: `websocket_rpc_loop` is disconnected from websocket server. Connection should be retried.
/// - This function spawns a thread and runs connection state machine in a loop.
///   - If current state is `Disconnected`: Spawns `websocket_rpc_loop` and sets state to `Connected`.
///   - If current state is `Connected`: Waits for `websocket_rpc_loop` thread to end and sets state to `Disconnected`.
pub fn monitor(
    url: String,
    channel_map: Arc<Mutex<HashMap<String, SyncSender<JsonRpcResponse>>>>,
    loop_handle: JoinHandle<()>,
    websocket_writer: Arc<Mutex<Writer<TcpStream>>>,
) -> Arc<Mutex<ConnectionState>> {
    let connection_state = Arc::new(Mutex::new(ConnectionState::Connected));
    let connection_state_clone = connection_state.clone();

    thread::spawn(move || {
        let mut connection_handle = Some(loop_handle);

        loop {
            let connection_state = *connection_state_clone
                .lock()
                .expect("Unable to acquire lock on connection state");

            let (new_connection_state, new_connection_handle) = match connection_state {
                ConnectionState::Disconnected => {
                    log::warn!("Websocket RPC is disconnected. Trying to reconnect");

                    match new_connection(&url) {
                        Err(err) => {
                            log::warn!("Websocket RPC reconnection failure: {:?}", err);
                            (ConnectionState::Disconnected, None)
                        }
                        Ok((new_websocket_reader, new_websocket_writer)) => {
                            log::info!("Websocket RPC successfully reconnected");

                            *websocket_writer
                                .lock()
                                .expect("Unable to acquire lock on websocket writer while reconnecting: Lock is poisoned") = new_websocket_writer;

                            let new_handle = spawn(
                                channel_map.clone(),
                                new_websocket_reader,
                                websocket_writer.clone(),
                            );

                            (ConnectionState::Connected, Some(new_handle))
                        }
                    }
                }
                ConnectionState::Connected => {
                    let _ = connection_handle.unwrap().join();
                    (ConnectionState::Disconnected, None)
                }
            };

            *connection_state_clone
                .lock()
                .expect("Unable to acquire lock on connection state") = new_connection_state;
            connection_handle = new_connection_handle;

            thread::sleep(MONITOR_RETRY_INTERVAL);
        }
    });

    connection_state
}

/// Deserializes message from websocket into `JsonRpcResponse`
#[inline]
fn parse_text(message: &str) -> Result<JsonRpcResponse> {
    serde_json::from_str(&message).chain(|| {
        (
            ErrorKind::DeserializationError,
            format!("Unable to deserialize websocket message: {}", message),
        )
    })
}

/// Deserializes message from websocket into `JsonRpcResponse`
#[inline]
fn parse_slice(message: &[u8]) -> Result<JsonRpcResponse> {
    serde_json::from_slice(message).chain(|| {
        (
            ErrorKind::DeserializationError,
            format!("Unable to deserialize websocket message: {:?}", message),
        )
    })
}

/// Handles websocket text message
#[inline]
fn handle_text(
    message: &str,
    channel_map: Arc<Mutex<HashMap<String, SyncSender<JsonRpcResponse>>>>,
) {
    log::trace!("Received text websocket message: {}", message);
    handle_json_response(parse_text(message), channel_map)
}

/// Handles websocket binary message
#[inline]
fn handle_slice(
    message: &[u8],
    channel_map: Arc<Mutex<HashMap<String, SyncSender<JsonRpcResponse>>>>,
) {
    log::trace!("Received binary websocket message: {:?}", message);
    handle_json_response(parse_slice(message), channel_map)
}

/// Handles parsed json response
fn handle_json_response(
    response: Result<JsonRpcResponse>,
    channel_map: Arc<Mutex<HashMap<String, SyncSender<JsonRpcResponse>>>>,
) {
    match response {
        Ok(response) => send_response(response, channel_map.clone()),
        Err(err) => {
            log::error!("{:?}", err);
        }
    }
}

/// Sends json response to appropriate channel
fn send_response(
    response: JsonRpcResponse,
    channel_map: Arc<Mutex<HashMap<String, SyncSender<JsonRpcResponse>>>>,
) {
    let sender = channel_map
        .lock()
        .expect("Unable to acquire lock on websocket channel map: Lock is poisoned")
        .remove(&response.id);

    if let Some(sender) = sender {
        log::debug!("Sending JSON-RPC response to channel");
        sender
            .send(response)
            .expect("Unable to send message on channel sender");
    } else {
        log::warn!("Received a websocket message with no configured handler");
    }
}

/// Silently sends pong message on websocket (does nothing in case of error)
fn send_pong(websocket_writer: Arc<Mutex<Writer<TcpStream>>>, data: Vec<u8>) {
    let pong = websocket_writer
        .lock()
        .expect("Unable to acquire lock on websocket writer")
        .send_message(&OwnedMessage::Pong(data));

    log::trace!("Received ping, sending pong: {:?}", pong);
}
