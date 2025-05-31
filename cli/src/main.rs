use switchbot_cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut cli = Cli::new_from_args();
    cli.run().await?;
    Ok(())
}
