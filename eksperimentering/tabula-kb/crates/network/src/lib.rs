//! P2P networking for the Tabula blockchain using libp2p.
//!
//! Features:
//! - Gossipsub for block and transaction propagation
//! - mDNS for local peer discovery

use std::collections::HashSet;
use std::time::Duration;

use futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId, ValidationMode},
    mdns,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Swarm, SwarmBuilder,
};
// Re-export for external use
pub use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use types::{Block, Hash, SignedTransaction};

/// Gossipsub topic for new blocks.
pub const BLOCKS_TOPIC: &str = "tabula/blocks/1";

/// Gossipsub topic for new transactions.
pub const TXS_TOPIC: &str = "tabula/txs/1";

// ============================================================================
// Network Messages
// ============================================================================

/// Messages broadcast via gossipsub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// New block announcement.
    NewBlock(Block),
    /// New transaction announcement.
    NewTransaction(SignedTransaction),
    /// Request block by hash (simple sync).
    RequestBlock(Hash),
    /// Block response.
    BlockResponse(Option<Block>),
    /// Request chain tip.
    RequestTip,
    /// Tip response (hash, height).
    TipResponse { hash: Hash, height: u64 },
}

// ============================================================================
// Network Events (sent to the node)
// ============================================================================

/// Events emitted by the network to the node.
#[derive(Debug)]
pub enum NetworkEvent {
    /// A new block was received from a peer.
    BlockReceived { block: Block, from: PeerId },
    /// A new transaction was received from a peer.
    TransactionReceived { tx: SignedTransaction, from: PeerId },
    /// A peer connected.
    PeerConnected(PeerId),
    /// A peer disconnected.
    PeerDisconnected(PeerId),
    /// Block request received.
    BlockRequested { hash: Hash, from: PeerId },
    /// Block response received.
    BlockResponseReceived { block: Option<Block>, from: PeerId },
    /// Tip request received.
    TipRequested { from: PeerId },
    /// Tip response received.
    TipResponseReceived { hash: Hash, height: u64, from: PeerId },
}

// ============================================================================
// Network Commands (sent from the node)
// ============================================================================

/// Commands sent to the network from the node.
#[derive(Debug)]
pub enum NetworkCommand {
    /// Broadcast a new block to all peers.
    BroadcastBlock(Block),
    /// Broadcast a new transaction to all peers.
    BroadcastTransaction(SignedTransaction),
    /// Request a block by hash (broadcasts to all peers).
    RequestBlock(Hash),
    /// Send a block response.
    SendBlockResponse(Option<Block>),
    /// Request the chain tip.
    RequestTip,
    /// Send tip response.
    SendTipResponse { hash: Hash, height: u64 },
}

// ============================================================================
// Network Behaviour
// ============================================================================

/// Combined network behaviour for Tabula.
#[derive(NetworkBehaviour)]
pub struct TabulaBehaviour {
    /// Gossipsub for pub/sub messaging.
    gossipsub: gossipsub::Behaviour,
    /// mDNS for local peer discovery.
    mdns: mdns::tokio::Behaviour,
}

// ============================================================================
// Network Configuration
// ============================================================================

/// Network configuration.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Address to listen on.
    pub listen_addr: Multiaddr,
    /// Bootstrap peers to connect to.
    pub bootstrap_peers: Vec<Multiaddr>,
    /// Enable mDNS for local discovery.
    pub enable_mdns: bool,
    /// Node keypair seed (for deterministic peer ID).
    pub keypair_seed: Option<[u8; 32]>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "/ip4/0.0.0.0/tcp/0".parse().unwrap(),
            bootstrap_peers: vec![],
            enable_mdns: true,
            keypair_seed: None,
        }
    }
}

impl NetworkConfig {
    /// Create config for a specific port.
    pub fn with_port(port: u16) -> Self {
        Self {
            listen_addr: format!("/ip4/0.0.0.0/tcp/{}", port).parse().unwrap(),
            ..Default::default()
        }
    }
}

// ============================================================================
// Network Service
// ============================================================================

/// Handle to the network service.
pub struct NetworkHandle {
    /// Our peer ID.
    pub peer_id: PeerId,
    /// Channel to send commands to the network.
    command_tx: mpsc::Sender<NetworkCommand>,
    /// Channel to receive events from the network.
    event_rx: mpsc::Receiver<NetworkEvent>,
}

impl NetworkHandle {
    /// Broadcast a new block.
    pub async fn broadcast_block(&self, block: Block) {
        let _ = self.command_tx.send(NetworkCommand::BroadcastBlock(block)).await;
    }

    /// Broadcast a new transaction.
    pub async fn broadcast_transaction(&self, tx: SignedTransaction) {
        let _ = self.command_tx.send(NetworkCommand::BroadcastTransaction(tx)).await;
    }

    /// Request a block from the network.
    pub async fn request_block(&self, hash: Hash) {
        let _ = self.command_tx.send(NetworkCommand::RequestBlock(hash)).await;
    }

    /// Send a block response.
    pub async fn send_block_response(&self, block: Option<Block>) {
        let _ = self.command_tx.send(NetworkCommand::SendBlockResponse(block)).await;
    }

    /// Request the chain tip.
    pub async fn request_tip(&self) {
        let _ = self.command_tx.send(NetworkCommand::RequestTip).await;
    }

    /// Send tip response.
    pub async fn send_tip_response(&self, hash: Hash, height: u64) {
        let _ = self.command_tx.send(NetworkCommand::SendTipResponse { hash, height }).await;
    }

    /// Receive the next network event.
    pub async fn recv(&mut self) -> Option<NetworkEvent> {
        self.event_rx.recv().await
    }

    /// Try to receive a network event without blocking.
    pub fn try_recv(&mut self) -> Option<NetworkEvent> {
        self.event_rx.try_recv().ok()
    }
}

// ============================================================================
// Network Service Implementation
// ============================================================================

/// Start the network service.
pub async fn start_network(config: NetworkConfig) -> Result<NetworkHandle, NetworkError> {
    // Create keypair
    let keypair = if let Some(seed) = config.keypair_seed {
        let secret = libp2p::identity::ed25519::SecretKey::try_from_bytes(seed.to_vec())
            .map_err(|e| NetworkError::KeypairError(e.to_string()))?;
        libp2p::identity::Keypair::from(libp2p::identity::ed25519::Keypair::from(secret))
    } else {
        libp2p::identity::Keypair::generate_ed25519()
    };

    let peer_id = PeerId::from(keypair.public());
    info!("Local peer ID: {}", peer_id);

    // Create gossipsub
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(ValidationMode::Strict)
        .message_id_fn(|msg: &gossipsub::Message| {
            // Use hash of data as message ID for deduplication
            let hash = crypto::sha256(&msg.data);
            MessageId::from(hash.to_vec())
        })
        .build()
        .map_err(|e| NetworkError::ConfigError(e.to_string()))?;

    let gossipsub = gossipsub::Behaviour::new(
        MessageAuthenticity::Signed(keypair.clone()),
        gossipsub_config,
    )
    .map_err(|e| NetworkError::ConfigError(e.to_string()))?;

    // Create mDNS
    let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)
        .map_err(|e| NetworkError::ConfigError(e.to_string()))?;

    // Create behaviour
    let behaviour = TabulaBehaviour { gossipsub, mdns };

    // Build swarm
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|e| NetworkError::TransportError(e.to_string()))?
        .with_behaviour(|_| behaviour)
        .map_err(|e| NetworkError::ConfigError(e.to_string()))?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Subscribe to topics
    let blocks_topic = IdentTopic::new(BLOCKS_TOPIC);
    let txs_topic = IdentTopic::new(TXS_TOPIC);
    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&blocks_topic)
        .map_err(|e| NetworkError::SubscriptionError(e.to_string()))?;
    swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&txs_topic)
        .map_err(|e| NetworkError::SubscriptionError(e.to_string()))?;

    // Listen on address
    swarm
        .listen_on(config.listen_addr.clone())
        .map_err(|e| NetworkError::ListenError(e.to_string()))?;

    // Connect to bootstrap peers
    for addr in &config.bootstrap_peers {
        match swarm.dial(addr.clone()) {
            Ok(_) => info!("Dialing bootstrap peer: {}", addr),
            Err(e) => warn!("Failed to dial {}: {}", addr, e),
        }
    }

    // Create channels
    let (command_tx, command_rx) = mpsc::channel(256);
    let (event_tx, event_rx) = mpsc::channel(256);

    // Spawn network task
    tokio::spawn(run_network_loop(swarm, command_rx, event_tx));

    Ok(NetworkHandle {
        peer_id,
        command_tx,
        event_rx,
    })
}

/// Main network event loop.
async fn run_network_loop(
    mut swarm: Swarm<TabulaBehaviour>,
    mut command_rx: mpsc::Receiver<NetworkCommand>,
    event_tx: mpsc::Sender<NetworkEvent>,
) {
    let blocks_topic = IdentTopic::new(BLOCKS_TOPIC);
    let txs_topic = IdentTopic::new(TXS_TOPIC);
    let mut known_peers: HashSet<PeerId> = HashSet::new();

    loop {
        tokio::select! {
            // Handle swarm events
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(TabulaBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, propagation_source, .. }
                    )) => {
                        // Decode and forward message
                        if let Ok(msg) = serde_cbor::from_slice::<GossipMessage>(&message.data) {
                            match msg {
                                GossipMessage::NewBlock(block) => {
                                    debug!("Received block {} from {}", block.header.height, propagation_source);
                                    let _ = event_tx.send(NetworkEvent::BlockReceived {
                                        block,
                                        from: propagation_source,
                                    }).await;
                                }
                                GossipMessage::NewTransaction(tx) => {
                                    debug!("Received transaction from {}", propagation_source);
                                    let _ = event_tx.send(NetworkEvent::TransactionReceived {
                                        tx,
                                        from: propagation_source,
                                    }).await;
                                }
                                GossipMessage::RequestBlock(hash) => {
                                    debug!("Block request from {}", propagation_source);
                                    let _ = event_tx.send(NetworkEvent::BlockRequested {
                                        hash,
                                        from: propagation_source,
                                    }).await;
                                }
                                GossipMessage::BlockResponse(block) => {
                                    debug!("Block response from {}", propagation_source);
                                    let _ = event_tx.send(NetworkEvent::BlockResponseReceived {
                                        block,
                                        from: propagation_source,
                                    }).await;
                                }
                                GossipMessage::RequestTip => {
                                    debug!("Tip request from {}", propagation_source);
                                    let _ = event_tx.send(NetworkEvent::TipRequested {
                                        from: propagation_source,
                                    }).await;
                                }
                                GossipMessage::TipResponse { hash, height } => {
                                    debug!("Tip response from {}: height {}", propagation_source, height);
                                    let _ = event_tx.send(NetworkEvent::TipResponseReceived {
                                        hash,
                                        height,
                                        from: propagation_source,
                                    }).await;
                                }
                            }
                        }
                    }
                    SwarmEvent::Behaviour(TabulaBehaviourEvent::Mdns(event)) => {
                        match event {
                            mdns::Event::Discovered(peers) => {
                                for (peer_id, addr) in peers {
                                    if known_peers.insert(peer_id) {
                                        info!("mDNS discovered peer: {} at {}", peer_id, addr);
                                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                        let _ = event_tx.send(NetworkEvent::PeerConnected(peer_id)).await;
                                    }
                                }
                            }
                            mdns::Event::Expired(peers) => {
                                for (peer_id, _) in peers {
                                    if known_peers.remove(&peer_id) {
                                        info!("mDNS peer expired: {}", peer_id);
                                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                                        let _ = event_tx.send(NetworkEvent::PeerDisconnected(peer_id)).await;
                                    }
                                }
                            }
                        }
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        if known_peers.insert(peer_id) {
                            info!("Connection established with {}", peer_id);
                            let _ = event_tx.send(NetworkEvent::PeerConnected(peer_id)).await;
                        }
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        if known_peers.remove(&peer_id) {
                            info!("Connection closed with {}", peer_id);
                            let _ = event_tx.send(NetworkEvent::PeerDisconnected(peer_id)).await;
                        }
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!("Listening on {}", address);
                    }
                    _ => {}
                }
            }

            // Handle commands from the node
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    NetworkCommand::BroadcastBlock(block) => {
                        let msg = GossipMessage::NewBlock(block);
                        if let Ok(data) = serde_cbor::to_vec(&msg) {
                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(blocks_topic.clone(), data) {
                                warn!("Failed to broadcast block: {}", e);
                            } else {
                                debug!("Broadcast block");
                            }
                        }
                    }
                    NetworkCommand::BroadcastTransaction(tx) => {
                        let msg = GossipMessage::NewTransaction(tx);
                        if let Ok(data) = serde_cbor::to_vec(&msg) {
                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(txs_topic.clone(), data) {
                                warn!("Failed to broadcast transaction: {}", e);
                            } else {
                                debug!("Broadcast transaction");
                            }
                        }
                    }
                    NetworkCommand::RequestBlock(hash) => {
                        let msg = GossipMessage::RequestBlock(hash);
                        if let Ok(data) = serde_cbor::to_vec(&msg) {
                            let _ = swarm.behaviour_mut().gossipsub.publish(blocks_topic.clone(), data);
                        }
                    }
                    NetworkCommand::SendBlockResponse(block) => {
                        let msg = GossipMessage::BlockResponse(block);
                        if let Ok(data) = serde_cbor::to_vec(&msg) {
                            let _ = swarm.behaviour_mut().gossipsub.publish(blocks_topic.clone(), data);
                        }
                    }
                    NetworkCommand::RequestTip => {
                        let msg = GossipMessage::RequestTip;
                        if let Ok(data) = serde_cbor::to_vec(&msg) {
                            let _ = swarm.behaviour_mut().gossipsub.publish(blocks_topic.clone(), data);
                        }
                    }
                    NetworkCommand::SendTipResponse { hash, height } => {
                        let msg = GossipMessage::TipResponse { hash, height };
                        if let Ok(data) = serde_cbor::to_vec(&msg) {
                            let _ = swarm.behaviour_mut().gossipsub.publish(blocks_topic.clone(), data);
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("keypair error: {0}")]
    KeypairError(String),
    #[error("config error: {0}")]
    ConfigError(String),
    #[error("transport error: {0}")]
    TransportError(String),
    #[error("listen error: {0}")]
    ListenError(String),
    #[error("subscription error: {0}")]
    SubscriptionError(String),
}

