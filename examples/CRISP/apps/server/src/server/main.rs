use crisp::server;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server::start()
}