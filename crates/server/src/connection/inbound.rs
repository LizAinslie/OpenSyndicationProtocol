use log::{debug, error, info};

use openssl::rand::rand_bytes;
use openssl::rsa::{Padding, Rsa};

use tokio::io;
use tokio::net::TcpStream;

use trust_dns_resolver::{TokioAsyncResolver};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};

use uuid::Uuid;

use osp_protocol::{ConnectionType, Protocol};
use osp_protocol::packet::{PacketDecoder, PacketEncoder};
use osp_protocol::packet::handshake::{HandshakePacketGuestToHost, HandshakePacketHostToGuest};
use osp_protocol::packet::transfer::{TransferPacketGuestToHost, TransferPacketHostToGuest};

pub struct InboundConnection<TState> {
    connection_type: ConnectionType,
    state: TState
}

pub struct HandshakeState {
    nonce: Uuid,
    protocol: Protocol<HandshakePacketGuestToHost, HandshakePacketHostToGuest>
}
pub struct TransferState {
    protocol: Protocol<TransferPacketGuestToHost, TransferPacketHostToGuest>
}

impl From<InboundConnection<HandshakeState>> for InboundConnection<TransferState> {
    fn from(value: InboundConnection<HandshakeState>) -> Self {
        InboundConnection {
            connection_type: value.connection_type,
            state: TransferState {
                protocol: value.state.protocol.map_codecs(
                    |_| {
                        PacketDecoder::new() // Transfer packet types implied!
                    },
                    |_| {
                        PacketEncoder::new()
                    }
                ),
            },
        }
    }
}

impl InboundConnection<HandshakeState> {
    pub fn with_stream(stream: TcpStream) -> io::Result<Self> {
        Ok(Self {
            connection_type: ConnectionType::Unknown,
            state: HandshakeState {
                nonce: Uuid::new_v4(),
                protocol: Protocol::with_stream(stream)?,
            }
        })
    }

    async fn send_close_err(&mut self, error_kind: io::ErrorKind, err: String) -> io::Error {
        error!("Closing connection with error: {}", err.clone());
        self.state.protocol.send_message(HandshakePacketHostToGuest::Close {
            can_continue: false,
            err: Some(err.clone()),
        }).await.unwrap();
        io::Error::new(error_kind, err)
    }

    pub async fn begin(&mut self) -> io::Result<()> {
        if let HandshakePacketGuestToHost::Hello { connection_type } = self.state.protocol.read_frame().await? {
            self.connection_type = connection_type;

            self.state.protocol.send_message(HandshakePacketHostToGuest::Acknowledge {
                ok: true,
                err: None
            }).await?;

            if let HandshakePacketGuestToHost::Identify { hostname } = self.state.protocol.read_frame().await? {
                // todo: check whitelist/blacklist
                info!("Looking up challenge record for {hostname}");
                let resolver = TokioAsyncResolver::tokio(
                    ResolverConfig::default(),
                    ResolverOpts::default());
                let txt_resp = resolver.txt_lookup(format!("_osp.{}", hostname)).await;
                match txt_resp {
                    Ok(txt_resp) => {
                        if let Some(record) = txt_resp.iter().next() {
                            info!("Challenge record found");
                            debug!("Challenge record: {record}");
                            let pub_key = Rsa::public_key_from_pem(record.to_string().as_bytes())?;

                            info!("Generating and encrypting challenge bytes");
                            let mut challenge_bytes = [0; 256];
                            rand_bytes(&mut challenge_bytes).unwrap();
                            let mut encrypted_challenge = vec![0u8; pub_key.size() as usize];
                            pub_key.public_encrypt(&challenge_bytes, &mut encrypted_challenge, Padding::PKCS1)?;

                            info!("Sending challenge bytes");
                            self.state.protocol.send_message(HandshakePacketHostToGuest::Challenge {
                                encrypted_challenge,
                                nonce: self.state.nonce,
                            }).await?;

                            if let HandshakePacketGuestToHost::Verify { challenge, nonce } = self.state.protocol.read_frame().await? {
                                info!("Received challenge verification");
                                if nonce != self.state.nonce {
                                    error!("Challenge response had invalid nonce. Expected: {} Actual: {}. Rejecting...", self.state.nonce, nonce);
                                    return Err(self.send_close_err(io::ErrorKind::InvalidData, "Invalid nonce".to_string()).await);
                                }

                                if challenge == challenge_bytes {
                                    info!("Challenge verification successful");
                                    self.state.protocol.send_message(HandshakePacketHostToGuest::Close {
                                        can_continue: true,
                                        err: None,
                                    }).await?;
                                    debug!("Sent success packet.");
                                    Ok(())
                                } else {
                                    error!("Challenge failed as bytes did not match. Rejecting...");
                                    return Err(self.send_close_err(io::ErrorKind::PermissionDenied, "Challenge failed".to_string()).await)
                                }
                            } else {
                                return Err(self.send_close_err(io::ErrorKind::InvalidInput, "Expected challenge verification packet".to_string()).await);
                            }
                        } else {
                            return Err(
                                self.send_close_err(
                                    io::ErrorKind::InvalidData,
                                    format!("Failed to resolve SRV record for {}. Is it located at _osp.{}?", hostname, hostname)
                                ).await
                            );
                        }
                    }
                    Err(e) => {
                        return Err(
                            self.send_close_err(
                                io::ErrorKind::Other,
                                format!(
                                    "Failed to resolve SRV record for {}. Is it located at _osp.{}?\n\nFurther Details: {}",
                                    hostname, hostname, e.to_string()
                                )
                            ).await
                        );
                    }
                }
            } else {
                return Err(self.send_close_err(io::ErrorKind::InvalidInput, "Expected identify packet".to_string()).await);
            }
        } else {
            return Err(self.send_close_err(io::ErrorKind::InvalidInput, "Expected hello packet".to_string()).await);
        }
    }
}