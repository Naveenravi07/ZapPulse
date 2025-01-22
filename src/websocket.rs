use crate::message::{Message, MessageKind};
use chrono::Local;
use futures_util::{stream::StreamExt, SinkExt};
use tokio_tungstenite::tungstenite::protocol::Message as TungSteniteMsg;
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

pub async fn start_websocket(
    url: String,
    tx: crossbeam_channel::Sender<Message>,
    rx: crossbeam_channel::Receiver<Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (ws_stream, _) = connect_async(url.into_client_request()?).await?;
    let (mut write, mut read) = ws_stream.split();

    let write_handle = tokio::spawn(async move {
        loop {
            if let Ok(msg) = rx.recv() {
                match msg.kind {
                    MessageKind::OUTGOING => {
                        let ws_msg = TungSteniteMsg::text(msg.content);
                        if let Err(e) = write.send(ws_msg).await {
                            eprintln!("Failed to send message: {}", e);
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    let read_handle = tokio::spawn(async move {
        while let Some(message) = read.next().await {
            if let Ok(msg) = message {
                let content = msg.to_string();
                if let Err(e) = tx.send(Message {
                    content,
                    kind: MessageKind::INCOMING,
                    time: Local::now(),
                }) {
                    eprintln!("Failed to send to channel: {}", e);
                    break;
                }
            }
        }
    });

    tokio::spawn(async move {
        let _ = tokio::join!(write_handle, read_handle);
    });

    Ok(())
}

