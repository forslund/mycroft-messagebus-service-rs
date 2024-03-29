use std::{
    collections::HashMap,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use simple_logger::SimpleLogger;

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;
use tungstenite::handshake::server::{Request, Response};
use tungstenite::http::StatusCode;

use rustcroft::config;
use log::{info, trace, LevelFilter};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;



async fn handle_connection(peer_map: PeerMap,
                           raw_stream: TcpStream,
                           addr: SocketAddr,
                           route: String) {
    info!("Incoming TCP connection from: {}", addr);
    let callback =  |req: &Request, res: Response|{
        if req.uri() != route.as_str() {
            let resp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Some(
                    "This is not the endpoint you're looking for".into()
                ))
                .unwrap();
            Err(resp)
        }
        else {
            Ok(res)
        }
    };
    let ws_stream = tokio_tungstenite::accept_hdr_async(
            raw_stream,
            callback
        ).await
        .expect("Error during the websocket handshake occurred");

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);

    let (outgoing,  incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        trace!("Received a message from {}: {}",
                 addr, msg.to_text().unwrap());
        if msg.is_text() || msg.is_binary() {

            let peers = peer_map.lock().unwrap();

            // We want to broadcast the message to everyone except ourselves.
            let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);

            for recp in broadcast_recipients {
                recp.unbounded_send(msg.clone()).unwrap();
            }
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    info!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}


#[tokio::main]
async fn main () -> Result<(), IoError> {
    SimpleLogger::new().with_level(LevelFilter::Info).env().init().unwrap();

    let cfg = config::ConfigStack::from_default().unwrap();
    let val = cfg.get(&["websocket", "host"]).unwrap();
    let host = val.as_str().unwrap();
    let port = cfg.get(&["websocket", "port"]).unwrap().as_i64().unwrap();
    let route = cfg.get(&["websocket", "route"]).unwrap()
        .as_str().unwrap().to_owned();
    let bind_dest = format!("{}:{}", host, port);
    info!("Starting at {}", bind_dest);

    let default = config::load_default().unwrap();
    let default_ws = &default["websocket"];

    info!("{}", default_ws["route"]);

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&bind_dest).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", bind_dest);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream,
                                       addr, route.clone()));
    }

    Ok(())
}
