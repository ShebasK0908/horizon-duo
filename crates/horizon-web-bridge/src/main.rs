use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Semaphore, mpsc},
};
use tokio_tungstenite::{
    accept_hdr_async_with_config,
    tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request, Response},
        protocol::WebSocketConfig,
    },
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
struct WorldRoute {
    path: String,
    upstream: SocketAddr,
    origins: Vec<String>,
}
impl WorldRoute {
    fn authorize(&self, path: &str, origin: Option<&str>) -> bool {
        path == self.path
            && origin.is_some_and(|origin| self.origins.iter().any(|allowed| allowed == origin))
    }
}
struct Options {
    bind: SocketAddr,
    route: WorldRoute,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut args = std::env::args().skip(1);
        let (mut world, mut upstream, mut bind) = (None, None, "127.0.0.1:8767".parse()?);
        let mut origins = Vec::new();
        while let Some(arg) = args.next() {
            let value = args.next().ok_or("Each option needs a value")?;
            match arg.as_str() {
                "--bind" => bind = value.parse()?,
                "--upstream" => upstream = Some(value.parse()?),
                "--world" => world = Some(value),
                "--origin" => origins.push(value),
                _ => return Err(format!("Unknown option {arg}").into()),
            }
        }
        let world = world.ok_or("Pass --world WORLD --upstream IP:PORT --origin ORIGIN")?;
        if world.is_empty()
            || world.len() > 64
            || !world
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("World id must contain 1 to 64 letters, numbers, or hyphens".into());
        }
        if origins.is_empty()
            || origins.iter().any(|o| {
                !(o.starts_with("http://") || o.starts_with("https://")) || o.contains('*')
            })
        {
            return Err("Pass at least one exact HTTP or HTTPS --origin".into());
        }
        Ok(Self {
            bind,
            route: WorldRoute {
                path: format!("/world/{world}"),
                upstream: upstream.ok_or("Pass --upstream IP:PORT")?,
                origins,
            },
        })
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::parse()?;
    let listener = TcpListener::bind(options.bind).await?;
    println!(
        "Horizon world bridge {}{}",
        listener.local_addr()?,
        options.route.path
    );
    let route = Arc::new(options.route);
    let slots = Arc::new(Semaphore::new(32));
    loop {
        let (stream, _) = listener.accept().await?;
        let Ok(slot) = slots.clone().try_acquire_owned() else {
            continue;
        };
        let route = route.clone();
        tokio::spawn(async move {
            let _slot = slot;
            if let Err(error) = connection(stream, route).await {
                eprintln!("World transport closed: {error}");
            }
        });
    }
}
async fn connection(stream: TcpStream, route: Arc<WorldRoute>) -> Result<()> {
    let check =
        |request: &Request, response: Response| -> std::result::Result<Response, ErrorResponse> {
            let origin = request
                .headers()
                .get("origin")
                .and_then(|value| value.to_str().ok());
            if route.authorize(request.uri().path(), origin) && request.uri().query().is_none() {
                Ok(response)
            } else {
                Err(Response::builder()
                    .status(403)
                    .body(Some("World or origin is not allowed".into()))
                    .unwrap())
            }
        };
    let config = WebSocketConfig::default()
        .write_buffer_size(64 << 10)
        .max_write_buffer_size(256 << 10)
        .max_message_size(Some(2 << 20))
        .max_frame_size(Some(2 << 20));
    let ws = tokio::time::timeout(
        Duration::from_secs(5),
        accept_hdr_async_with_config(stream, check, Some(config)),
    )
    .await??;
    let tcp =
        tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(route.upstream)).await??;
    tcp.set_nodelay(true)?;
    let (mut to_web, mut from_web) = ws.split();
    let (mut from_server, mut to_server) = tcp.into_split();
    let (control, mut controls) = mpsc::channel(4);
    let upstream = async {
        while let Some(message) = from_web.next().await {
            match message? {
                Message::Binary(bytes) => to_server.write_all(&bytes).await?,
                Message::Ping(bytes) => control.send(Message::Pong(bytes)).await?,
                Message::Close(_) => break,
                Message::Text(_) => {
                    return Err::<(), Box<dyn std::error::Error + Send + Sync>>(
                        "Only binary Minecraft bytes are accepted".into(),
                    );
                }
                _ => {}
            }
        }
        Ok(())
    };
    let downstream = async {
        let mut buffer = vec![0; 64 << 10];
        loop {
            tokio::select! {
                bytes = from_server.read(&mut buffer) => {
                    let len = bytes?;
                    if len == 0 {break;}
                    to_web.send(Message::Binary(buffer[..len].to_vec().into())).await?;
                }
                message = controls.recv() => {
                    if let Some(message) = message {to_web.send(message).await?;}
                }
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::select! {result = upstream => result?, result = downstream => result?};
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_configured_world_and_exact_origin_are_authorized() {
        let route = WorldRoute {
            path: "/world/test".into(),
            upstream: "127.0.0.1:25565".parse().unwrap(),
            origins: vec!["http://127.0.0.1:8766".into()],
        };
        assert!(route.authorize("/world/test", Some("http://127.0.0.1:8766")));
        assert!(!route.authorize("/world/other", Some("http://127.0.0.1:8766")));
        assert!(!route.authorize("/world/test", Some("http://127.0.0.1:8766.evil.test")));
        assert!(!route.authorize("/world/test", None));
    }
}
