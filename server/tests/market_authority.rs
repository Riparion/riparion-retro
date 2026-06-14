//! End-to-end check of the authoritative shared market: a human trade routed
//! through the world actor moves the shared price and is broadcast to clients.
//! Drives the actor directly (no socket) — the WebSocket layer is a thin JSON
//! shim over exactly these commands.

use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

use kaintuck_engine::market_link::Side;
use kaintuck_engine::net::ServerMsg;
use kaintuck_server::actor::{spawn, Command};

#[tokio::test]
async fn a_human_buy_moves_and_broadcasts_the_shared_price() {
    // A slow tick so bot trading/relaxation doesn't swamp the human order in the
    // window we measure; one bot just to prove bots coexist.
    let handle = spawn(1, 100_000);
    let mut events = handle.events.subscribe();

    // Join.
    let (jtx, jrx) = oneshot::channel();
    handle.cmd.send(Command::Join { reply: jtx }).await.unwrap();
    let (_id, snapshot) = jrx.await.unwrap();

    let (town, good) = (3usize, 1usize);
    let before = snapshot.mids[town][good];

    // A big buy should lift the price.
    let (ttx, trx) = oneshot::channel();
    handle
        .cmd
        .send(Command::Trade {
            town,
            good,
            side: Side::Buy,
            qty: 200,
            reply: ttx,
        })
        .await
        .unwrap();
    let fill = trx.await.unwrap().expect("trade should fill");
    assert!((fill - before).abs() < 1e-6, "fill is the pre-trade mid");

    // The trade broadcasts an immediate delta for that town with a higher mid.
    let mut saw_higher = false;
    for _ in 0..8 {
        match timeout(Duration::from_secs(2), events.recv()).await {
            Ok(Ok(ServerMsg::MarketDelta { town: t, mids })) if t == town => {
                if mids[good] > before {
                    saw_higher = true;
                    break;
                }
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(saw_higher, "buy did not raise the broadcast shared price");
}
