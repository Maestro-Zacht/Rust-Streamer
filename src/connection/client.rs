use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeHandler};
use std::{io, thread};

pub struct ConnectionClient {
    ws_handler: NodeHandler<()>,
}

impl ConnectionClient {
    pub fn new<T: AsRef<str>>(
        ip: T,
        mut on_disconnect: impl FnMut() -> () + Send + 'static,
    ) -> io::Result<Self> {
        let (ws_handler, listener) = node::split::<()>();

        ws_handler
            .network()
            .connect(Transport::Ws, format!("{}:9000", ip.as_ref()))?;

        thread::spawn(move || {
            listener.for_each(move |event| match event.network() {
                NetEvent::Connected(..) => println!("Connected"),
                NetEvent::Accepted(..) => unreachable!(),
                NetEvent::Message(..) => println!("Message"),
                NetEvent::Disconnected(_) => on_disconnect(),
            });
        });

        Ok(Self { ws_handler })
    }
}

impl Drop for ConnectionClient {
    fn drop(&mut self) {
        self.ws_handler.stop();
    }
}
