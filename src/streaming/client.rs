use byte_slice_cast::*;
use std::{io, sync::Arc};

use crate::connection::client::ConnectionClient;
use gstreamer::{self as gst, element_error, glib, prelude::*};
use gstreamer_app as gst_app;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamingClientError {
    #[error("GStreamer init error: {0}")]
    GStreamerInitError(#[from] glib::Error),

    #[error("GStreamer element error: {0}")]
    GStreamerElementCreationError(#[from] glib::BoolError),

    #[error("GStreamer state change error: {0}")]
    GStreamerStateChangeError(#[from] gst::StateChangeError),

    #[error("Websocket error: {0}")]
    WebsocketError(#[from] io::Error),
}

pub struct StreamingClient {
    pipeline: Arc<gst::Pipeline>,
    _connection_client: ConnectionClient,
}

impl StreamingClient {
    pub fn new<T: AsRef<str>>(
        ip: T,
        mut image_parser: impl FnMut(&[u8]) + Send + 'static,
    ) -> Result<Self, StreamingClientError> {
        gst::init()?;

        let source = gst::ElementFactory::make("udpsrc")
            .property("port", 9001)
            .build()?;

        let depay = gst::ElementFactory::make("rtph264depay").build()?;
        let decode = gst::ElementFactory::make("decodebin").build()?;
        let videoscale = gst::ElementFactory::make("videoscale").build()?;
        let capsfilter2 = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-raw")
                    .field("width", 400) // TODO dynamic scaling
                    .field("height", 400)
                    .build(),
            )
            .build()?;
        let convert = gst::ElementFactory::make("videoconvert").build()?;
        let jpegenc = gst::ElementFactory::make("jpegenc").build()?;
        let sink = gst_app::AppSink::builder()
            .max_buffers(3)
            .caps(&gst::Caps::builder("image/jpeg").build())
            .build();

        let pipeline = gst::Pipeline::with_name("recv-pipeline");

        pipeline.add_many(&[
            &source,
            &depay,
            &decode,
            &videoscale,
            &capsfilter2,
            &convert,
            &jpegenc,
            sink.upcast_ref(),
        ])?;

        source.link_filtered(
            &depay,
            &gst::Caps::builder("application/x-rtp")
                .field("media", "video")
                .field("clock-rate", 90000)
                .field("encoding-name", "H264")
                .field("payload", 96)
                .build(),
        )?;
        depay.link(&decode)?;

        let videoscale_weak = videoscale.downgrade();
        decode.connect_pad_added(move |_, src_pad| {
            let sink_pad = match videoscale_weak.upgrade() {
                None => return,
                Some(s) => s.static_pad("sink").unwrap(),
            };
            src_pad.link(&sink_pad).unwrap();
        });

        videoscale.link(&capsfilter2)?;
        capsfilter2.link(&convert)?;

        convert.link(&jpegenc)?;
        jpegenc.link(&sink)?;

        let pipeline = Arc::new(pipeline);

        let pipeline_clone = pipeline.clone();
        let connection_client = ConnectionClient::new(ip, move || {
            pipeline_clone.set_state(gst::State::Null).unwrap();
            // TODO better close connection
        })?;

        sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or_else(|| {
                        element_error!(
                            appsink,
                            gst::ResourceError::Failed,
                            ("Failed to get buffer from appsink")
                        );

                        gst::FlowError::Error
                    })?;

                    let map = buffer.map_readable().map_err(|_| {
                        element_error!(
                            appsink,
                            gst::ResourceError::Failed,
                            ("Failed to map buffer readable")
                        );

                        gst::FlowError::Error
                    })?;

                    let samples = map.as_slice_of::<u8>().map_err(|_| {
                        element_error!(
                            appsink,
                            gst::ResourceError::Failed,
                            ("Failed to interpret buffer as bytes")
                        );

                        gst::FlowError::Error
                    })?;

                    image_parser(samples);

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        Ok(Self {
            pipeline,
            _connection_client: connection_client,
        })
    }

    pub fn start(&self) -> Result<(), StreamingClientError> {
        Ok(self.pipeline.set_state(gst::State::Playing).map(|_| ())?)
    }
}

impl Drop for StreamingClient {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
