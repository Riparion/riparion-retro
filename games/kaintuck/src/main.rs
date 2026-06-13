mod app;
mod engine;
mod leaderboard;
mod storage;
mod ui;

fn main() {
    dioxus::launch(app::App);
}
