use crate::{
    Error, Result,
    buf::{Reader, Writer},
    crypto::{self, Cfb8},
    frame,
    packets::{Empty, configuration as cfg, handshake, login, play},
    registry::Dynamic,
    rsa::RsaPublicKey,
};

const MAX_PENDING: usize = 16 << 20;

#[derive(Debug)]
pub enum Phase {
    Login,
    Configuration {
        registries: Dynamic,
        profile: login::GameProfile,
    },
    Play,
    Closed,
}
#[derive(Debug)]
pub enum Event {
    Send(Vec<u8>),
    Joined {
        registries: Dynamic,
        profile: login::GameProfile,
    },
    Packet(Box<play::Clientbound>),
}
pub struct ClientSession {
    pub phase: Phase,
    pending: Vec<u8>,
    compression: Option<usize>,
    incoming: Option<Cfb8>,
    outgoing: Option<Cfb8>,
    identity: [u8; 32],
}
impl ClientSession {
    pub fn begin(
        host: String,
        port: u16,
        name: String,
        identity: [u8; 32],
    ) -> Result<(Self, Vec<Event>)> {
        if name.is_empty()
            || name.len() > 16
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(Error::new(
                "Use a player name of 1 to 16 letters, numbers, or underscores",
            ));
        }
        let session = Self {
            phase: Phase::Login,
            pending: Vec::new(),
            compression: None,
            incoming: None,
            outgoing: None,
            identity,
        };
        let intention = handshake::Serverbound::Intention(handshake::Intention {
            protocol: crate::PROTOCOL_VERSION,
            host,
            port,
            intent: handshake::LOGIN,
        })
        .encode();
        let hello = login::Serverbound::Hello(login::ServerHello {
            uuid: crypto::offline_uuid(&name),
            name,
        })
        .encode();
        Ok((
            session,
            vec![
                Event::Send(frame::encode(&intention, None)?),
                Event::Send(frame::encode(&hello, None)?),
            ],
        ))
    }
    fn send(&mut self, body: &[u8]) -> Result<Event> {
        let mut bytes = frame::encode(body, self.compression)?;
        if let Some(cipher) = &mut self.outgoing {
            cipher.encrypt(&mut bytes);
        }
        Ok(Event::Send(bytes))
    }
    pub fn send_play(&mut self, packet: play::Serverbound) -> Result<Event> {
        if !matches!(self.phase, Phase::Play) {
            return Err(Error::new("The session is not in play"));
        }
        self.send(&packet.encode())
    }
    pub fn receive(&mut self, mut bytes: Vec<u8>) -> Result<Vec<Event>> {
        if matches!(self.phase, Phase::Closed) {
            return Err(Error::new("The session is closed"));
        }
        if self.pending.len().saturating_add(bytes.len()) > MAX_PENDING {
            self.phase = Phase::Closed;
            return Err(Error::new("Server exceeded the pending packet limit"));
        }
        if let Some(cipher) = &mut self.incoming {
            cipher.decrypt(&mut bytes);
        }
        self.pending.extend_from_slice(&bytes);
        let mut events = Vec::new();
        loop {
            let frame = match frame::split(&mut self.pending) {
                Ok(Some(frame)) => frame,
                Ok(None) => break,
                Err(error) => {
                    self.phase = Phase::Closed;
                    return Err(error);
                }
            };
            let phase = std::mem::replace(&mut self.phase, Phase::Closed);
            let body = frame::decompress(frame, self.compression)?;
            let mut reader = Reader::new(&body);
            let id = reader.varint()?;
            let payload = &body[reader.position()..];
            self.phase = match phase {
                Phase::Login => match login::Clientbound::decode(id, payload)? {
                    login::Clientbound::Compression(packet) => {
                        self.compression = usize::try_from(packet.threshold).ok();
                        Phase::Login
                    }
                    login::Clientbound::LoginFinished(packet) => {
                        events.push(
                            self.send(&login::Serverbound::LoginAcknowledged(Empty).encode())?,
                        );
                        events.push(
                            self.send(
                                &cfg::Serverbound::ClientInformation(cfg::ClientInformation::new(
                                    4,
                                ))
                                .encode(),
                            )?,
                        );
                        let mut brand = Writer::new();
                        brand.string("horizon");
                        events.push(
                            self.send(
                                &cfg::Serverbound::CustomPayload(cfg::CustomPayload {
                                    channel: "minecraft:brand".into(),
                                    data: brand.data,
                                })
                                .encode(),
                            )?,
                        );
                        Phase::Configuration {
                            registries: Dynamic::default(),
                            profile: packet.profile,
                        }
                    }
                    login::Clientbound::Hello(packet) => {
                        if packet.should_authenticate {
                            return Err(Error::new(
                                "This browser client needs an offline-mode Horizon server; account authentication is not implemented",
                            ));
                        }
                        let key = RsaPublicKey::from_der(&packet.public_key)?;
                        let secret = crypto::generate_secret()?;
                        events.push(
                            self.send(
                                &login::Serverbound::Key(login::Key {
                                    key: key.encrypt(&secret)?,
                                    challenge: key.encrypt(&packet.challenge)?,
                                })
                                .encode(),
                            )?,
                        );
                        self.incoming = Some(Cfb8::new(&secret, &secret));
                        self.outgoing = Some(Cfb8::new(&secret, &secret));
                        Phase::Login
                    }
                    login::Clientbound::CustomQuery(packet) => {
                        events.push(
                            self.send(
                                &login::Serverbound::CustomQueryAnswer(login::CustomQueryAnswer {
                                    transaction: packet.transaction,
                                    payload: None,
                                })
                                .encode(),
                            )?,
                        );
                        Phase::Login
                    }
                    login::Clientbound::CookieRequest(packet) => {
                        events.push(
                            self.send(
                                &login::Serverbound::CookieResponse(login::CookieResponse {
                                    key: packet.key,
                                    payload: None,
                                })
                                .encode(),
                            )?,
                        );
                        Phase::Login
                    }
                    login::Clientbound::Disconnect(packet) => {
                        return Err(Error::new(packet.reason_json));
                    }
                    login::Clientbound::Unknown(..) => Phase::Login,
                },
                Phase::Configuration {
                    mut registries,
                    profile,
                } => {
                    let reply = match cfg::Clientbound::decode(id, payload)? {
                        cfg::Clientbound::RegistryData(packet) => {
                            registries.add(packet);
                            None
                        }
                        cfg::Clientbound::UpdateTags(packet) => {
                            registries.add_tags(&packet);
                            None
                        }
                        cfg::Clientbound::KeepAlive(packet) => {
                            Some(cfg::Serverbound::KeepAlive(packet))
                        }
                        cfg::Clientbound::Ping(packet) => Some(cfg::Serverbound::Pong(packet)),
                        cfg::Clientbound::SelectKnownPacks(_) => {
                            Some(cfg::Serverbound::SelectKnownPacks(cfg::SelectKnownPacks {
                                packs: vec![],
                            }))
                        }
                        cfg::Clientbound::CookieRequest(packet) => {
                            Some(cfg::Serverbound::CookieResponse(cfg::CookieResponse {
                                key: packet.key,
                                payload: None,
                            }))
                        }
                        cfg::Clientbound::CustomPayload(packet)
                            if packet.channel == "horizon:block_registry" =>
                        {
                            Some(cfg::Serverbound::CustomPayload(cfg::CustomPayload {
                                channel: packet.channel,
                                data: self.identity.to_vec(),
                            }))
                        }
                        cfg::Clientbound::ResourcePackPush(packet) => {
                            if packet.required {
                                return Err(Error::new(
                                    "This server requires a resource pack; browser server-pack import is not implemented",
                                ));
                            }
                            Some(cfg::Serverbound::ResourcePack(cfg::ResourcePackResponse {
                                id: packet.id,
                                action: 1,
                            }))
                        }
                        cfg::Clientbound::CodeOfConduct(_) => {
                            return Err(Error::new(
                                "This server requires code-of-conduct approval; browser approval is not implemented",
                            ));
                        }
                        cfg::Clientbound::Disconnect(packet) => {
                            return Err(Error::new(format!(
                                "Server disconnected: {:?}",
                                packet.reason
                            )));
                        }
                        cfg::Clientbound::FinishConfiguration(_) => {
                            events.push(
                                self.send(&cfg::Serverbound::FinishConfiguration(Empty).encode())?,
                            );
                            events.push(Event::Joined {
                                registries,
                                profile,
                            });
                            self.phase = Phase::Play;
                            continue;
                        }
                        _ => None,
                    };
                    if let Some(reply) = reply {
                        events.push(self.send(&reply.encode())?);
                    }
                    Phase::Configuration {
                        registries,
                        profile,
                    }
                }
                Phase::Play => {
                    match play::Clientbound::decode(id, payload)? {
                        play::Clientbound::KeepAlive(packet) => {
                            events.push(self.send(&play::Serverbound::KeepAlive(packet).encode())?)
                        }
                        play::Clientbound::Ping(packet) => {
                            events.push(self.send(&play::Serverbound::Pong(packet).encode())?)
                        }
                        play::Clientbound::Disconnect(packet) => {
                            return Err(Error::new(format!(
                                "Server disconnected: {:?}",
                                packet.reason
                            )));
                        }
                        packet => events.push(Event::Packet(Box::new(packet))),
                    }
                    Phase::Play
                }
                Phase::Closed => return Err(Error::new("The session is closed")),
            };
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_and_coalesced_packets_cross_compression_and_phase_boundaries() {
        let (mut session, start) =
            ClientSession::begin("localhost".into(), 25565, "Browser".into(), [0; 32]).unwrap();
        assert_eq!(start.len(), 2);
        let mut bytes = frame::encode(
            &login::Clientbound::Compression(login::Compression { threshold: 16 }).encode(),
            None,
        )
        .unwrap();
        bytes.extend(
            frame::encode(
                &login::Clientbound::LoginFinished(login::LoginFinished {
                    profile: login::GameProfile {
                        uuid: 1,
                        name: "Browser".into(),
                        properties: vec![],
                    },
                    session: 2,
                })
                .encode(),
                Some(16),
            )
            .unwrap(),
        );
        bytes.extend(
            frame::encode(
                &cfg::Clientbound::FinishConfiguration(Empty).encode(),
                Some(16),
            )
            .unwrap(),
        );
        bytes.extend(
            frame::encode(
                &play::Clientbound::KeepAlive(cfg::KeepAlive { id: 99 }).encode(),
                Some(16),
            )
            .unwrap(),
        );
        assert!(session.receive(bytes[..1].to_vec()).unwrap().is_empty());
        let events = session.receive(bytes[1..].to_vec()).unwrap();
        assert!(matches!(session.phase, Phase::Play));
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::Joined { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::Send(_)))
                .count(),
            5
        );
    }
    #[test]
    fn malformed_frames_close_the_session() {
        let (mut session, _) =
            ClientSession::begin("localhost".into(), 25565, "Browser".into(), [0; 32]).unwrap();
        assert!(session.receive(vec![0x80; 3]).is_err());
        assert!(matches!(session.phase, Phase::Closed));
    }
}
