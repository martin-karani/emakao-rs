#[tokio::main]
async fn main() -> anyhow::Result<()> {
    emakao::bootstrap::run().await
}
