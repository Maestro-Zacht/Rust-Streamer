use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeHandler};
use std::{io, thread};

pub struct ConnectionClient {
    thread_handle: thread::JoinHandle<()>,
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

        let thread_handle = thread::spawn(move || {
            listener.for_each(move |event| match event.network() {
                NetEvent::Connected(..) => println!("Connected"),
                NetEvent::Accepted(..) => unreachable!(),
                NetEvent::Message(..) => println!("Message"),
                NetEvent::Disconnected(_) => on_disconnect(),
            });
        });

        Ok(Self {
            thread_handle,
            ws_handler,
        })
    }

    pub fn stop(self) {
        self.ws_handler.stop();
        self.thread_handle.join().unwrap();
        println!("Connection client stopped");
    }
}
