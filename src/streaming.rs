pub mod client;
pub mod server;

pub enum Streaming {
    Client(client::StreamingClient),
    Server(server::StreamingServer),
}

impl Streaming {
    pub fn new_client<T: AsRef<str>>(ip: T) -> Result<Self, client::StreamingClientError> {
        client::StreamingClient::new(ip).map(Streaming::Client)
    }

    pub fn new_server() -> Result<Self, server::StreamingServerError> {
        server::StreamingServer::new().map(Streaming::Server)
    }
}
