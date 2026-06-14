mod app;
// The game logic now lives in the standalone, headless `kaintuck-engine` crate;
// alias it as `engine` so every `crate::engine::…` path across the UI resolves
// unchanged.
pub use kaintuck_engine as engine;
mod leaderboard;
mod net_client;
mod storage;
mod ui;

fn main() {
    dioxus::launch(app::App);
}
