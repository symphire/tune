use crate::domain::{ConversationId, MessageId};
use crate::infra::network::ws_api_v1::WS_CHAT_URL;
use crate::infra::network::ws_api_v1::{
    C2SCommand, ChatContent, ChatMessageSend, S2CEvent, SendMessage,
};
use crate::infra::network::WsWorker;
use crate::port::network::WithGeneration;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::fs;
use std::io::BufReader;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{http, Message};
use tokio_tungstenite::{connect_async_tls_with_config, MaybeTlsStream, WebSocketStream};
use tracing::{trace, warn};

pub struct RealWsWorker {
    pub generation: u64,
    pub to_sender: UnboundedSender<C2SCommand>,
    pub watcher_handle: JoinHandle<()>,
}

impl RealWsWorker {
    pub async fn try_new(
        generation: u64,
        access_token: String,
        from_receiver: UnboundedSender<WithGeneration<S2CEvent>>,
    ) -> anyhow::Result<Self> {
        // region Create connection
        let cert_file = &mut BufReader::new(fs::File::open("certs/dev_cert.pem")?);
        let certs = rustls_pemfile::certs(cert_file).collect::<Result<Vec<_>, _>>()?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in certs {
            root_store.add(cert)?
        }

        let _ = rustls::crypto::ring::default_provider().install_default();

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = tokio_tungstenite::Connector::Rustls(Arc::new(config));

        let url = url::Url::parse(WS_CHAT_URL)?;
        let mut request = url.into_client_request()?;
        request.headers_mut().insert(
            http::header::AUTHORIZATION,
            http::HeaderValue::from_str(format!("Bearer {}", access_token).clone().as_str())?,
        );

        let (ws_stream, _) =
            connect_async_tls_with_config(request, None, false, Some(connector)).await?;
        let (mut to_server, mut from_server) = ws_stream.split();
        // endregion

        // region Create sender and receiver
        let (to_sender, from_app) = unbounded_channel();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let sender_handle = tokio::spawn(sender(from_app, to_server, shutdown_rx.clone()));
        let receiver_handle = tokio::spawn(receiver(
            generation,
            from_server,
            from_receiver,
            shutdown_rx,
        ));
        let watcher_handle = tokio::spawn(watcher(sender_handle, receiver_handle, shutdown_tx));
        // endregion

        Ok(Self {
            generation,
            to_sender,
            watcher_handle,
        })
    }
}

// region helpers
async fn sender(
    mut from_app: UnboundedReceiver<C2SCommand>,
    mut to_server: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            Some(message) = from_app.recv() => {
                let _ = to_server.send(Message::Text(serde_json::to_string(&message).unwrap().into())).await;
            }
            _ = shutdown.changed() => break,
        }
    }
}

async fn receiver(
    generation: u64,
    mut from_server: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    mut from_receiver: UnboundedSender<WithGeneration<S2CEvent>>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            Some(message) = from_server.next() => {
                let message = match message {
                    Ok(Message::Text(body)) => body,
                    Ok(Message::Close(_)) => break,
                    Ok(_) => continue,
                    Err(_) => break,
                };

                trace!("received message: {}", message);
                match serde_json::from_str(&message) {
                    Ok(message) => {
                        let message = WithGeneration {
                            generation,
                            result: message,
                        };
                        trace!("decoded message: {:?}", message);
                        let _ = from_receiver.send(message);
                    }
                    Err(e) => {
                        tracing::error!("malformed message: {}", e);
                        break
                    },
                }
            }
            _ = shutdown.changed() => break,
        }
    }
}

async fn watcher(
    sender_handle: JoinHandle<()>,
    receiver_handle: JoinHandle<()>,
    shutdown: watch::Sender<bool>,
) {
    let _ = tokio::select! {
        result = sender_handle => {
            warn!("Sender task ended");
            let _ = shutdown.send(true);
        },
        result = receiver_handle => {
            warn!("Receiver task ended");
            let _ = shutdown.send(true);
        }
    };
}
// endregion

#[async_trait::async_trait]
impl WsWorker for RealWsWorker {
    async fn send_message(
        &self,
        message_id: MessageId,
        conversation_id: ConversationId,
        content: String,
    ) -> anyhow::Result<()> {
        let message = C2SCommand::ChatMessageSend(ChatMessageSend {
            conversation_id,
            message_id,
            content,
        });
        self.to_sender.send(message)?;
        Ok(())
    }
}
