use switchbot_cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_ok() {
        env_logger::init();
    } else {
        env_logger::Builder::new()
            .format_timestamp(None)
            .format_target(false)
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    let mut cli = Cli::new_from_args();
    cli.run().await?;
    Ok(())
}
