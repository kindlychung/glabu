#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    glabu::cli::execute().await?;
    Ok(())
}
