use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};

use tokio::{
    select,
    sync::{mpsc, Mutex, Notify, OwnedSemaphorePermit},
};

use crate::packet::{BasePacket, PacketReader, PacketWriter};

#[derive(Debug)]
struct SessionIdManager {
    session_id: AtomicU64,
}

// this is temporary because other services run in 32-bit mode
// once those are rewritten this will be removed
pub const MAX_SESS_ID_VALUE: u32 = 0x80000000;

impl SessionIdManager {
    pub const fn new() -> Self {
        SessionIdManager {
            session_id: AtomicU64::new(1),
        }
    }

    pub fn get_next_session_id(&mut self) -> u64 {
        let mut session_id_as_int = self.session_id.load(std::sync::atomic::Ordering::Relaxed);
        if session_id_as_int + 2 == MAX_SESS_ID_VALUE as u64 {
            self.session_id
                .store(1, std::sync::atomic::Ordering::Relaxed);
            return session_id_as_int;
        } else {
            session_id_as_int = self
                .session_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        session_id_as_int + 1
    }
}

#[derive(Debug)]
pub struct RPCCall {
    session_id: u64,
    result: Mutex<Option<BasePacket>>,
    finished_notification: Arc<Notify>,
}

impl RPCCall {
    pub async fn wait_for_result(&self, timeout: std::time::Duration) -> anyhow::Result<()> {
        let notify = self.finished_notification.clone();

        select! {
            _ = notify.notified() => {
                Ok(())
            }

            _ = tokio::time::sleep(timeout) => {
                Err(anyhow::anyhow!("Timeout waiting for rpc call"))
            }
        }
    }

    pub async fn get_result(&self) -> Option<BasePacket> {
        self.result.lock().await.take()
    }
}

#[derive(Debug)]
pub struct RPCManager {
    session_id_manager: SessionIdManager,
    rpc_calls: HashMap<u64, Arc<RPCCall>>,
    rpc_send_tx: mpsc::Sender<BasePacket>,
}

impl RPCManager {
    pub fn new(rpc_send_tx: mpsc::Sender<BasePacket>) -> Self {
        RPCManager {
            session_id_manager: SessionIdManager::new(),
            rpc_calls: HashMap::new(),
            rpc_send_tx,
        }
    }

    pub async fn check_packet_for_rpc_response(&mut self, packet: &BasePacket) -> bool {
        if let Some(sess_id_u32) = packet.read_session_id() {
            let session_id = sess_id_u32 as u64;
            if self.rpc_calls.contains_key(&session_id) {
                return true;
            }

            return false;
        }

        false
    }

    pub async fn make_sync_call(&mut self, mut packet: BasePacket) -> anyhow::Result<Arc<RPCCall>> {
        let session_id = self.session_id_manager.get_next_session_id();
        let rpc_call = Arc::new(RPCCall {
            session_id,
            result: Mutex::new(None),
            finished_notification: Arc::new(Notify::new()),
        });

        self.rpc_calls.insert(session_id, rpc_call.clone());

        packet.write_session_id(session_id as u32)?;
        packet.build_packet()?;

        if let Err(err) = self.rpc_send_tx.send(packet).await {
            return Err(anyhow::anyhow!("Error sending rpc call: {:?}", err));
        }

        Ok(rpc_call)
    }

    pub async fn handle_rpc_reply(&mut self, packet: BasePacket) -> anyhow::Result<()> {
        if let Some(sess_id_u32) = packet.read_session_id() {
            let session_id = sess_id_u32 as u64;
            if let Some(rpc_call) = self.rpc_calls.get(&session_id) {
                let rpc_call = rpc_call.clone();
                let mut rpc_result = rpc_call.result.lock().await;
                *rpc_result = Some(packet);

                rpc_call.finished_notification.notify_one();

                return Ok(());
            }
        }

        Err(anyhow::anyhow!("No matching rpc call found"))
    }
}
