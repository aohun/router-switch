fn main() {
    let _guard = session::tokio_runtime().enter();
    ui::run();
}
