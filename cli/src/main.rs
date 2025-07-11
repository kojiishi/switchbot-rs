use std::io::Write;

use switchbot_cli::Cli;

#[tokio::main]
async fn main() {
    init_logger();

    let mut cli = Cli::new_from_args();
    if let Err(error) = cli.run().await {
        log::error!("{error}");
    }
}

fn init_logger() {
    // If `RUST_LOG` is set, initialize the `env_logger` in its default config.
    if std::env::var("RUST_LOG").is_ok() {
        env_logger::init();
        return;
    }

    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .format(|buf, record| match record.level() {
            log::Level::Info => writeln!(buf, "{}", record.args()),
            _ => {
                let style = buf.default_level_style(record.level());
                writeln!(buf, "{style}{}{style:#}: {}", record.level(), record.args())
            }
        })
        .target(env_logger::Target::Stdout)
        .init();
}
