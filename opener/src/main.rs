use clap::Parser;
use env_logger::Env;

/// Client program for opener controller
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Address of service: <ip> | <name>
    #[clap(short, long, value_parser)]
    address: String,

    /// Port of service
    #[clap(short, long, default_value_t = 4000)]
    port: u16,

    /// Serial number of controller
    #[clap(short, long, value_parser)]
    serial: String,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let args = Args::parse();

    opener::run(&args.address, args.port, &args.serial).await;
}
