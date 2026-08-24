//! A deliberately narrow relay: authenticated presence and connection signalling only.
//! It does not have screen capture, input injection, or unattended-password code.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use ed25519_dalek::{Signature, VerifyingKey};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, Semaphore, mpsc},
    time::{interval, timeout},
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    identity::device_id_from_public_key,
    protocol::{self, ErrorCode, Message, RejectReason},
};

const REQUEST_TTL: Duration = Duration::from_secs(60);
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(10);
const MAX_PENDING_REQUESTS: usize = 10_000;
const MAX_PENDING_PER_CLIENT: usize = 4;
const MAX_CONNECTIONS: usize = 512;
const MAX_AUTH_FAILURES: u8 = 5;
const AUTH_FAILURE_WINDOW: Duration = Duration::from_secs(60);
const AUTH_BLOCK_DURATION: Duration = Duration::from_secs(300);

#[derive(Clone)]
struct RegisteredPeer {
    tx: mpsc::Sender<Message>,
    connection_id: Uuid,
}

#[derive(Clone, Copy)]
struct Registration {
    device_id: u32,
    connection_id: Uuid,
}

#[derive(Clone)]
struct PendingRequest {
    client_device_id: u32,
    client_connection_id: Uuid,
    host_device_id: u32,
    host_connection_id: Uuid,
    expires_at: Instant,
}

#[derive(Clone, Copy)]
struct AuthenticationFailures {
    count: u8,
    window_started: Instant,
    blocked_until: Option<Instant>,
}

#[derive(Default)]
struct Registry {
    peers: HashMap<u32, RegisteredPeer>,
    pending: HashMap<Uuid, PendingRequest>,
    authentication_failures: HashMap<IpAddr, AuthenticationFailures>,
}

pub struct Relay {
    listener: TcpListener,
    registry: Arc<Mutex<Registry>>,
    connection_slots: Arc<Semaphore>,
}

impl Relay {
    pub async fn bind(address: SocketAddr) -> Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(address).await?,
            registry: Arc::new(Mutex::new(Registry::default())),
            connection_slots: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        })
    }

    pub async fn serve(self) -> Result<()> {
        let mut cleanup = interval(CLEANUP_INTERVAL);
        loop {
            tokio::select! {
                _ = cleanup.tick() => prune_expired(&self.registry).await,
                accepted = self.listener.accept() => {
                    let (stream, address) = match accepted {
                        Ok(pair) => pair,
                        Err(error) => {
                            warn!(%error, "relay accept failed; continuing");
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                    };
                    let Ok(permit) = Arc::clone(&self.connection_slots).try_acquire_owned() else {
                        warn!(%address, "connection limit reached; dropping peer");
                        drop(stream);
                        continue;
                    };
                    let registry = Arc::clone(&self.registry);
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(error) = handle_connection(stream, address.ip(), registry).await {
                            warn!(%address, %error, "relay peer disconnected with error");
                        }
                    });
                }
            }
        }
    }
}

fn send(tx: &mpsc::Sender<Message>, message: Message) -> bool {
    match tx.try_send(message) {
        Ok(()) => true,
        Err(mpsc::error::TrySendError::Full(_)) => {
            warn!("peer backlogged; dropping connection");
            false
        }
        Err(mpsc::error::TrySendError::Closed(_)) => false,
    }
}

async fn handle_connection(
    stream: TcpStream,
    peer_ip: IpAddr,
    registry: Arc<Mutex<Registry>>,
) -> Result<()> {
    stream.set_nodelay(true)?;
    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<Message>(32);
    let write_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            protocol::write_message(&mut writer, &message).await?;
        }
        Ok::<(), protocol::ProtocolError>(())
    });

    let connection_id = Uuid::new_v4();
    let mut registration = None;
    let result = handle_messages(
        &mut reader,
        &tx,
        Arc::clone(&registry),
        peer_ip,
        connection_id,
        &mut registration,
    )
    .await;
    unregister(&registry, registration).await;
    drop(tx);
    match write_task.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!(%error, "relay writer failed"),
        Err(error) => warn!(%error, "relay writer task failed"),
    }
    result
}

async fn handle_messages<R>(
    reader: &mut R,
    tx: &mpsc::Sender<Message>,
    registry: Arc<Mutex<Registry>>,
    peer_ip: IpAddr,
    connection_id: Uuid,
    registration: &mut Option<Registration>,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let hello = match timeout(HELLO_TIMEOUT, protocol::read_message(reader)).await {
        Ok(Ok(message)) => message,
        Ok(Err(error)) => return Err(error.into()),
        Err(_) => {
            warn!("peer timed out before HELLO");
            return Ok(());
        }
    };
    match hello {
        Message::Hello {
            protocol_version, ..
        } => {
            if protocol::require_compatible(protocol_version).is_err() {
                send(
                    tx,
                    Message::Error {
                        code: ErrorCode::IncompatibleProtocol,
                        message: "JREMOTE/2 is required".into(),
                    },
                );
                return Ok(());
            }
        }
        _ => return Ok(()),
    }

    let nonce: [u8; 32] = rand::random();
    if !send(
        tx,
        Message::Challenge {
            nonce: nonce.to_vec(),
        },
    ) {
        return Ok(());
    }

    loop {
        let message = match timeout(IDLE_TIMEOUT, protocol::read_message(reader)).await {
            Ok(Ok(message)) => message,
            Ok(Err(error)) => return Err(error.into()),
            Err(_) => {
                warn!("peer timed out while idle");
                return Ok(());
            }
        };
        let keep_open = match message {
            Message::Register {
                device_id,
                public_key,
                signature,
            } => {
                if registration.is_some() {
                    send(tx, invalid("already registered"))
                } else if authentication_is_blocked(&registry, peer_ip).await {
                    send(
                        tx,
                        authentication_failed(
                            "too many failed authentication attempts; try again later",
                        ),
                    )
                } else {
                    register(
                        tx,
                        &registry,
                        peer_ip,
                        connection_id,
                        &nonce,
                        device_id,
                        public_key,
                        signature,
                        registration,
                    )
                    .await
                }
            }
            Message::Lookup { device_id } => {
                if !require_current_registration(tx, &registry, *registration).await {
                    false
                } else {
                    let online = registry.lock().await.peers.contains_key(&device_id);
                    send(tx, Message::LookupResult { device_id, online })
                }
            }
            Message::ConnectRequest {
                request_id,
                from_device_id,
                target_device_id,
            } => {
                connect_request(
                    tx,
                    &registry,
                    *registration,
                    request_id,
                    from_device_id,
                    target_device_id,
                )
                .await
            }
            response @ Message::ConnectAccept { request_id }
            | response @ Message::ConnectReject { request_id, .. } => {
                connect_response(tx, &registry, *registration, request_id, response).await
            }
            Message::Ping { nonce } => send(tx, Message::Pong { nonce }),
            _ => send(
                tx,
                invalid("message is not valid in relay signalling state"),
            ),
        };
        if !keep_open {
            return Ok(());
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn register(
    tx: &mpsc::Sender<Message>,
    registry: &Mutex<Registry>,
    peer_ip: IpAddr,
    connection_id: Uuid,
    nonce: &[u8; 32],
    device_id: u32,
    public_key: Vec<u8>,
    signature: Vec<u8>,
    registration: &mut Option<Registration>,
) -> bool {
    let Ok(public_key) = <[u8; 32]>::try_from(public_key.as_slice()) else {
        record_authentication_failure(registry, peer_ip).await;
        return send(tx, authentication_failed("invalid public key"));
    };
    let Ok(signature) = <[u8; 64]>::try_from(signature.as_slice()) else {
        record_authentication_failure(registry, peer_ip).await;
        return send(tx, authentication_failed("invalid signature"));
    };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&public_key) else {
        record_authentication_failure(registry, peer_ip).await;
        return send(tx, authentication_failed("invalid public key"));
    };
    if !(100_000_000..1_000_000_000).contains(&device_id)
        || device_id != device_id_from_public_key(&public_key)
        || verifying_key
            .verify_strict(
                &protocol::registration_payload(nonce, device_id),
                &Signature::from_bytes(&signature),
            )
            .is_err()
    {
        record_authentication_failure(registry, peer_ip).await;
        return send(tx, authentication_failed("registration proof is invalid"));
    }

    let mut state = registry.lock().await;
    state.authentication_failures.remove(&peer_ip);
    let previous = state.peers.insert(
        device_id,
        RegisteredPeer {
            tx: tx.clone(),
            connection_id,
        },
    );
    drop(state);
    *registration = Some(Registration {
        device_id,
        connection_id,
    });
    if let Some(previous) = previous {
        send(
            &previous.tx,
            Message::Error {
                code: ErrorCode::NotRegistered,
                message: "this authenticated identity registered from a newer connection".into(),
            },
        );
    }
    info!(device_id, "authenticated endpoint registered");
    send(tx, Message::RegisterOk { device_id })
}

async fn authentication_is_blocked(registry: &Mutex<Registry>, peer_ip: IpAddr) -> bool {
    let now = Instant::now();
    registry
        .lock()
        .await
        .authentication_failures
        .get(&peer_ip)
        .and_then(|failures| failures.blocked_until)
        .is_some_and(|blocked_until| blocked_until > now)
}

async fn record_authentication_failure(registry: &Mutex<Registry>, peer_ip: IpAddr) {
    let now = Instant::now();
    let mut state = registry.lock().await;
    let failures = state
        .authentication_failures
        .entry(peer_ip)
        .or_insert(AuthenticationFailures {
            count: 0,
            window_started: now,
            blocked_until: None,
        });
    if now.duration_since(failures.window_started) > AUTH_FAILURE_WINDOW {
        *failures = AuthenticationFailures {
            count: 0,
            window_started: now,
            blocked_until: None,
        };
    }
    failures.count = failures.count.saturating_add(1);
    if failures.count >= MAX_AUTH_FAILURES {
        failures.blocked_until = Some(now + AUTH_BLOCK_DURATION);
        warn!(%peer_ip, "temporarily blocked after repeated authentication failures");
    }
}

async fn require_current_registration(
    tx: &mpsc::Sender<Message>,
    registry: &Mutex<Registry>,
    registration: Option<Registration>,
) -> bool {
    let Some(registration) = registration else {
        return send(
            tx,
            Message::Error {
                code: ErrorCode::NotRegistered,
                message: "register before sending this message".into(),
            },
        );
    };
    let current = registry
        .lock()
        .await
        .peers
        .get(&registration.device_id)
        .is_some_and(|peer| peer.connection_id == registration.connection_id);
    if current {
        true
    } else {
        send(
            tx,
            Message::Error {
                code: ErrorCode::NotRegistered,
                message: "registration was superseded by a newer connection".into(),
            },
        )
    }
}

async fn connect_request(
    tx: &mpsc::Sender<Message>,
    registry: &Mutex<Registry>,
    registration: Option<Registration>,
    request_id: Uuid,
    from_device_id: u32,
    target_device_id: u32,
) -> bool {
    if !require_current_registration(tx, registry, registration).await {
        return false;
    }
    let registration = registration.expect("checked above");
    if from_device_id != registration.device_id || from_device_id == target_device_id {
        return send(tx, invalid("invalid connection request"));
    }

    let outcome = {
        let mut state = registry.lock().await;
        prune_registry(&mut state);
        if state.pending.contains_key(&request_id) {
            RequestOutcome::ToRequester(invalid("request ID is already in use"))
        } else if state.pending.len() >= MAX_PENDING_REQUESTS
            || state
                .pending
                .values()
                .filter(|request| request.client_connection_id == registration.connection_id)
                .count()
                >= MAX_PENDING_PER_CLIENT
        {
            RequestOutcome::ToRequester(Message::ConnectReject {
                request_id,
                reason: RejectReason::RateLimited,
            })
        } else if let Some(host) = state.peers.get(&target_device_id) {
            let host_connection_id = host.connection_id;
            let host_tx = host.tx.clone();
            state.pending.insert(
                request_id,
                PendingRequest {
                    client_device_id: from_device_id,
                    client_connection_id: registration.connection_id,
                    host_device_id: target_device_id,
                    host_connection_id,
                    expires_at: Instant::now() + REQUEST_TTL,
                },
            );
            RequestOutcome::ToHost(host_tx)
        } else {
            RequestOutcome::ToRequester(Message::ConnectReject {
                request_id,
                reason: RejectReason::TargetOffline,
            })
        }
    };
    match outcome {
        RequestOutcome::ToHost(host_tx) => send(
            &host_tx,
            Message::ConnectRequest {
                request_id,
                from_device_id,
                target_device_id,
            },
        ),
        RequestOutcome::ToRequester(message) => send(tx, message),
    }
}

enum RequestOutcome {
    ToHost(mpsc::Sender<Message>),
    ToRequester(Message),
}

async fn connect_response(
    tx: &mpsc::Sender<Message>,
    registry: &Mutex<Registry>,
    registration: Option<Registration>,
    request_id: Uuid,
    response: Message,
) -> bool {
    if !require_current_registration(tx, registry, registration).await {
        return false;
    }
    let registration = registration.expect("checked above");
    let outcome = {
        let mut state = registry.lock().await;
        prune_registry(&mut state);
        let Some(request) = state.pending.get(&request_id).cloned() else {
            return send(tx, invalid("unknown or expired connection request"));
        };
        if request.host_device_id != registration.device_id
            || request.host_connection_id != registration.connection_id
        {
            return send(
                tx,
                invalid("connection request is not authorized for this device"),
            );
        }
        state.pending.remove(&request_id);
        state
            .peers
            .get(&request.client_device_id)
            .filter(|peer| peer.connection_id == request.client_connection_id)
            .map(|peer| peer.tx.clone())
    };
    outcome.is_none_or(|client_tx| send(&client_tx, response))
}

async fn unregister(registry: &Mutex<Registry>, registration: Option<Registration>) {
    let Some(registration) = registration else {
        return;
    };
    let mut state = registry.lock().await;
    state.pending.retain(|_, request| {
        request.client_connection_id != registration.connection_id
            && request.host_connection_id != registration.connection_id
    });
    if state
        .peers
        .get(&registration.device_id)
        .is_some_and(|peer| peer.connection_id == registration.connection_id)
    {
        state.peers.remove(&registration.device_id);
        info!(device_id = registration.device_id, "endpoint unregistered");
    }
}

async fn prune_expired(registry: &Mutex<Registry>) {
    let mut state = registry.lock().await;
    prune_registry(&mut state);
}

fn prune_registry(state: &mut Registry) {
    let now = Instant::now();
    state.pending.retain(|_, request| {
        request.expires_at > now
            && state
                .peers
                .get(&request.client_device_id)
                .is_some_and(|peer| peer.connection_id == request.client_connection_id)
            && state
                .peers
                .get(&request.host_device_id)
                .is_some_and(|peer| peer.connection_id == request.host_connection_id)
    });
}

fn invalid(message: impl Into<String>) -> Message {
    Message::Error {
        code: ErrorCode::InvalidMessage,
        message: message.into(),
    }
}

fn authentication_failed(message: impl Into<String>) -> Message {
    Message::Error {
        code: ErrorCode::AuthenticationFailed,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn repeated_authentication_failures_temporarily_block_an_ip() {
        let registry = Mutex::new(Registry::default());
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        for _ in 0..MAX_AUTH_FAILURES {
            record_authentication_failure(&registry, ip).await;
        }
        assert!(authentication_is_blocked(&registry, ip).await);
    }
}
