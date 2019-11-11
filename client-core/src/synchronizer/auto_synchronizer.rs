//! auto sync network handler
//! (todo) make upper json rpc wrapper

use super::auto_sync_core::AutoSynchronizerCore;
use super::auto_sync_data::{
    AutoSyncDataShared, AutoSyncInfo, AutoSyncQueue, AutoSyncSendQueue, AutoSyncSendQueueShared,
    NetworkState, RestartCommand, WalletInfos, WebsocketState,
};
use super::auto_sync_data::{MyQueue, CMD_SUBSCRIBE};
use crate::BlockHandler;
use client_common::tendermint::Client;
use client_common::{Result, Storage};
use serde_json::json;
use std::sync::{Arc, Mutex};

use futures::future::Future;
use futures::sink::Sink;
use futures::stream::Stream;
use log;
use std::thread;
use websocket::result::WebSocketError;
use websocket::ClientBuilder;
use websocket::OwnedMessage;
/** constanct connection
using ws://localhost:26657/websocket
*/
pub struct AutoSynchronizer {
    /// core
    pub core: Option<MyQueue>,
    /// websocket url
    websocket_url: String,
    /// to network
    send_queue: AutoSyncSendQueueShared,
    /// to core
    data: AutoSyncDataShared,
}

/// handling web-socket
impl AutoSynchronizer {
    /// send json via channel
    pub fn send_json(sendq: &AutoSyncQueue, data: serde_json::Value) {
        sendq
            .send(OwnedMessage::Text(serde_json::to_string(&data).unwrap()))
            .unwrap();
    }
    /// get send queue
    pub fn get_send_queue(&mut self) -> Option<std::sync::mpsc::Sender<OwnedMessage>> {
        assert!(self.core.is_some());
        Some(self.core.as_mut().unwrap().clone())
    }
    /// create auto sync
    pub fn new(websocket_url: String, data: AutoSyncDataShared) -> Self {
        Self {
            /// core
            core: None,
            /// websocket url
            websocket_url,
            send_queue: Arc::new(Mutex::new(AutoSyncSendQueue::new())),
            data,
        }
    }

    /// set connected
    pub fn set_state(&self, state: NetworkState<WebsocketState>) {
        let mut data = self.data.lock().unwrap();
        data.info.state = state;
    }

    pub fn clear_info(&self) {
        let mut data = self.data.lock().unwrap();
        data.info = AutoSyncInfo::default();

        let json = json!(RestartCommand {
            id: "restart".to_string(),
        });
        if data.send_queue_to_core.is_some() {
            AutoSynchronizer::send_json(
                &data
                    .send_queue_to_core
                    .as_ref()
                    .expect("clear_info get send_queue_to_core"),
                json,
            );
        }
    }

    /// launch core thread
    pub fn run<S: Storage + Clone + 'static, C: Client + 'static, H: BlockHandler + 'static>(
        &mut self,
        client: C,
        storage: S,
        block_handler: H,
        data: AutoSyncDataShared,
    ) {
        let mut core = AutoSynchronizerCore::new(
            self.send_queue.clone(),
            storage,
            client,
            block_handler,
            WalletInfos::new(),
            data,
        );
        // save send_queue to communicate with core
        self.core = Some(core.get_queue());

        let _child = thread::spawn(move || {
            core.start();
        });

        assert!(self.core.is_some());
    }

    // to close channel, all senders of the channel should be dropped
    // currently it has two senders
    // 1. stream
    // 2. sending queue

    fn close_connection(&self) {
        let mut data = self
            .send_queue
            .lock()
            .expect("autosync close connection send-queue lock");

        data.queue = None;
    }

    fn process_text(&self, a: &str) -> std::result::Result<(), ()> {
        let j: serde_json::Value = serde_json::from_str(&a).map_err(|_e| {})?;
        if j["error"].is_null() {
            if let Some(core) = self.core.as_ref() {
                core.send(OwnedMessage::Text(a.into())).map_err(|_e| {})?;
            }
            Ok(())
        } else {
            Err(())
        }
    }

    fn do_run_network(&mut self) {
        let mut connected = false;
        self.set_state(NetworkState::Ready);
        let channel = futures::sync::mpsc::channel(0);
        // tx, rx
        let (channel_tx, channel_rx) = channel;
        {
            let data = &mut self.send_queue.lock().unwrap();
            data.queue = Some(channel_tx.clone());
        }
        let mut runtime = tokio::runtime::current_thread::Builder::new()
            .build()
            .expect("get tokio builder");
        // get synchronous sink
        let mut channel_sink = channel_tx.clone().wait();
        drop(channel_tx);

        let runner = ClientBuilder::new(&self.websocket_url)
            .expect("client-builder new")
            .add_protocol("rust-websocket")
            .async_connect_insecure()
            .and_then(|(duplex, _)| {
                log::info!("successfully connected to {}", self.websocket_url);
                connected = true;
                self.set_state(NetworkState::Connected(WebsocketState::ReadyProcess));
                channel_sink
                    .send(OwnedMessage::Text(CMD_SUBSCRIBE.to_string()))
                    .expect("send to channel sink");
                let (sink, stream) = duplex.split();
                drop(channel_sink);

                stream
                    .filter_map(|message| match message {
                        OwnedMessage::Text(a) => {
                            if self.process_text(&a).is_err() {
                                log::warn!("close connection in auto-sync");
                                self.close_connection();
                            }
                            None
                        }
                        OwnedMessage::Binary(_a) => None,
                        OwnedMessage::Close(e) => Some(OwnedMessage::Close(e)),
                        OwnedMessage::Ping(d) => Some(OwnedMessage::Pong(d)),
                        _ => None,
                    })
                    .select(channel_rx.map_err(|_| WebSocketError::NoDataAvailable))
                    .forward(sink)
            });
        self.set_state(NetworkState::Connecting);
        match runtime.block_on(runner) {
            Ok(_a) => {
                log::info!("connection gracefully closed");
            }
            Err(b) => {
                // write log only after connection is made
                if connected {
                    log::warn!("connection closed error {}", b);
                }
            }
        }
        self.set_state(NetworkState::Disconnected);
        std::thread::sleep(std::time::Duration::from_millis(2000));
        self.clear_info();
    }

    /// activate tokio websocket
    pub fn run_network(&mut self) -> Result<()> {
        loop {
            self.do_run_network();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::super::auto_sync_data::AutoSyncData;
    use super::*;

    #[test]
    fn check_state_change() {
        let sync = AutoSynchronizer::new(
            "ws://localhost:26657/websocket".into(),
            Arc::new(Mutex::new(AutoSyncData::default())),
        );
        sync.set_state(NetworkState::Connecting);
        {
            let data = sync.data.lock().unwrap();
            assert!(data.info.state == NetworkState::Connecting);
        }
        sync.set_state(NetworkState::Disconnected);
        {
            let data = sync.data.lock().unwrap();
            assert!(data.info.state == NetworkState::Disconnected);
        }
        sync.set_state(NetworkState::Connected(WebsocketState::ReadyProcess));
        {
            let data = sync.data.lock().unwrap();
            assert!(data.info.state == NetworkState::Connected(WebsocketState::ReadyProcess));
        }
    }
    #[test]
    fn check_reconnection() {
        // normal port is 26657, used invalid port to test retrying
        let mut sync = AutoSynchronizer::new(
            "ws://localhost:6657/websocket".into(),
            Arc::new(Mutex::new(AutoSyncData::default())),
        );
        // perform two times
        // check whether reconnection trying is valid
        for _i in 0..2 {
            {
                let data = sync.data.lock().unwrap();
                assert!(data.info.state == NetworkState::Ready);
            }
            sync.do_run_network();
            {
                let data = sync.data.lock().unwrap();
                assert!(data.info.state == NetworkState::Ready);
            }
        }
    }
}
