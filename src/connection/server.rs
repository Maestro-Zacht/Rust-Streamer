use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeHandler};
use std::{io, thread};

pub struct ConnectionServer {
    ws_handler: NodeHandler<()>,
}

impl ConnectionServer {
    pub fn new(
        mut on_connect: impl FnMut(&str) -> () + Send + 'static,
        mut on_disconnect: impl FnMut(&str) -> () + Send + 'static,
    ) -> io::Result<Self> {
        let (ws_handler, listener) = node::split::<()>();

        ws_handler.network().listen(Transport::Ws, "0.0.0.0:9000")?;

        thread::spawn(move || {
            listener.for_each(move |event| match event.network() {
                NetEvent::Connected(..) => unreachable!(),
                NetEvent::Accepted(endpoint, _) => {
                    let ip = endpoint.addr().ip().to_string();
                    on_connect(&ip);
                }
                NetEvent::Message(..) => println!("Message"),
                NetEvent::Disconnected(endpoint) => {
                    let ip = endpoint.addr().ip().to_string();
                    on_disconnect(&ip);
                }
            });
        });

        Ok(Self { ws_handler })
    }
}

impl Drop for ConnectionServer {
    fn drop(&mut self) {
        self.ws_handler.stop();
    }
}
