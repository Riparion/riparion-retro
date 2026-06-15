//! A real over-the-socket end-to-end test: stand up the full axum server on a
//! loopback port and talk to it with an actual WebSocket client (tokio-
//! tungstenite), exactly as the browser client does. This exercises the
//! `/ws` upgrade, the JSON framing in `ws.rs`, and the authoritative market —
//! the layer the in-process actor test can't reach.

use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use kaintuck_engine::market_link::Side;
use kaintuck_engine::net::{ClientMsg, ServerMsg};

async fn next_server_msg<S>(ws: &mut S) -> Option<ServerMsg>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while let Some(Ok(frame)) = ws.next().await {
        if let Message::Text(t) = frame {
            if let Ok(m) = serde_json::from_str::<ServerMsg>(&t) {
                return Some(m);
            }
        }
    }
    None
}

#[tokio::test]
async fn websocket_join_then_trade_round_trips_over_a_real_socket() {
    // Stand up the real server on an ephemeral port.
    let handle = kaintuck_server::actor::spawn(2, 150);
    let app = kaintuck_server::app(handle);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Connect a real WebSocket client.
    let url = format!("ws://{addr}/ws");
    let (mut ws, _resp) = connect_async(&url).await.expect("ws connect");

    // Join.
    let join = serde_json::to_string(&ClientMsg::Join {
        handle: "tester".into(),
    })
    .unwrap();
    ws.send(Message::Text(join.into())).await.unwrap();

    // Welcome carries a full snapshot — but the server is also broadcasting a
    // market delta every tick, so Welcome can arrive interleaved with deltas.
    // Scan for it (capturing the pre-trade mid from its snapshot).
    let (town, good) = (3usize, 1usize);
    let mut before = None;
    for _ in 0..50 {
        match next_server_msg(&mut ws).await {
            Some(ServerMsg::Welcome { participant_id, snapshot }) => {
                assert!(participant_id >= 1_000_000, "human ids start high");
                assert!(snapshot.towns() >= 1, "snapshot has towns");
                before = Some(snapshot.mids[town][good]);
                break;
            }
            Some(_) => {} // tick deltas before our Welcome — fine
            None => break,
        }
    }
    let before = before.expect("never received Welcome over the socket");

    // Send a big buy and expect a TradeFill at the pre-trade mid.
    let order = serde_json::to_string(&ClientMsg::TradeOrder {
        town,
        good,
        side: Side::Buy,
        qty: 300,
    })
    .unwrap();
    ws.send(Message::Text(order.into())).await.unwrap();

    // We should see our fill, and the shared price for that town should rise in a
    // broadcast delta. Both can be interleaved with other towns' tick-deltas.
    let mut saw_fill = false;
    let mut saw_higher = false;
    for _ in 0..50 {
        match next_server_msg(&mut ws).await {
            Some(ServerMsg::TradeFill { town: t, good: g, mid, .. }) if t == town && g == good => {
                assert!((mid - before).abs() < 1e-6, "fill reports the pre-trade mid");
                saw_fill = true;
            }
            Some(ServerMsg::MarketDelta { town: t, mids }) if t == town => {
                if mids[good] > before {
                    saw_higher = true;
                }
            }
            Some(_) => {}
            None => break,
        }
        if saw_fill && saw_higher {
            break;
        }
    }
    assert!(saw_fill, "never received our TradeFill over the socket");
    assert!(saw_higher, "buy did not raise the broadcast shared price");

    // Clean close.
    let _ = ws.send(Message::Close(None)).await;
}
