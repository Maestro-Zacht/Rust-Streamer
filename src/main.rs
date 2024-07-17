use rust_streamer::streaming::Streaming;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Send,
    Recv(ServerArgs),
}

#[derive(Args, Debug)]
struct ServerArgs {
    ip: String,
}

fn main() {
    let cli = Cli::parse();

    let streaming = match cli.command {
        Commands::Send => Streaming::new_server().unwrap(),
        Commands::Recv(ServerArgs { ip }) => Streaming::new_client(ip).unwrap(),
    };

    match streaming {
        Streaming::Client(client) => {
            client.start().unwrap();
            println!("Client started");
            std::thread::sleep(std::time::Duration::from_secs(100));
            client.stop().unwrap();
        }
        Streaming::Server(server) => {
            server.start().unwrap();
            println!("Server started");
            std::thread::sleep(std::time::Duration::from_secs(100));
            server.close().unwrap();
        }
    }
}
