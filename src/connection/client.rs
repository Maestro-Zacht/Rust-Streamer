use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeHandler};
use std::sync::mpsc::channel;
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

        let (tx, rx) = channel();

        thread::spawn(move || {
            listener.for_each(move |event| match event.network() {
                NetEvent::Connected(_, success) => {
                    tx.send(success).unwrap();
                    if success {
                        println!("Connected");
                    } else {
                        println!("Failed to connect");
                    }
                }
                NetEvent::Accepted(..) => unreachable!(),
                NetEvent::Message(..) => println!("Message"),
                NetEvent::Disconnected(_) => on_disconnect(),
            });
        });

        if rx.recv().unwrap() {
            Ok(Self { ws_handler })
        } else {
            ws_handler.stop();
            Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "Failed to connect",
            ))
        }
    }
}

impl Drop for ConnectionClient {
    fn drop(&mut self) {
        self.ws_handler.stop();
    }
}
