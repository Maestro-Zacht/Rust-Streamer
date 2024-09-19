pub mod client;
pub mod server;

pub enum Streaming {
    Client(client::StreamingClient),
    Server(server::StreamingServer),
}

impl Streaming {
    pub fn new_client<T: AsRef<str>>(
        ip: T,
        image_parser: impl FnMut(&[u8]) + Send + 'static,
        save_stream: bool,
    ) -> Result<Self, client::StreamingClientError> {
        client::StreamingClient::new(ip, image_parser, save_stream).map(Streaming::Client)
    }

    pub fn new_server(
        image_parser: impl FnMut(&[u8]) + Send + 'static,
    ) -> Result<Self, server::StreamingServerError> {
        server::StreamingServer::new(image_parser).map(Streaming::Server)
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Streaming::Client(client) => client.start().map_err(|e| e.into()),
            Streaming::Server(server) => server.start().map_err(|e| e.into()),
        }
    }
}
