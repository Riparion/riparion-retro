mod app;
mod engine;
mod storage;
mod ui;

fn main() {
    dioxus::launch(app::App);
}
