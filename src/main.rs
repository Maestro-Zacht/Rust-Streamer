use std::sync::{Arc, Mutex};

use image::ImageFormat;
use rust_streamer::streaming::Streaming;

use clap::{Args, Parser, Subcommand};
use eframe::egui::{self, Color32};

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

struct MyApp {
    _streaming: Streaming,
    current_image: Arc<Mutex<Option<egui::ColorImage>>>,
    texture: Option<egui::TextureHandle>,
}

impl MyApp {
    fn new() -> Self {
        // TODO not use a fake streaming
        let current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
            [200, 200],
            Color32::BLACK,
        ))));
        let image_clone = current_image.clone();
        let streaming = Streaming::new_server(move |bytes| {
            let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                .unwrap()
                .to_rgba8();

            let size = [image.width() as usize, image.height() as usize];
            let image = egui::ColorImage::from_rgba_unmultiplied(size, &image);

            // println!("Received image with size {:?}", size);

            *image_clone.lock().unwrap() = Some(image);
        })
        .unwrap();
        streaming.start().unwrap();
        Self {
            _streaming: streaming,
            current_image,
            texture: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
            ui.label("This is an example app");

            let mut data = self.current_image.lock().unwrap();
            if let Some(image) = data.take() {
                self.texture = Some(ui.ctx().load_texture("image", image, Default::default()));
            }
            drop(data);

            if let Some(texture) = &self.texture {
                ui.add(egui::Image::from_texture(texture).shrink_to_fit());
            }
        });
    }
}

fn main() {
    // let cli = Cli::parse();

    // let streaming = match cli.command {
    //     Commands::Send => Streaming::new_server(|_| {}).unwrap(),
    //     Commands::Recv(ServerArgs { ip }) => Streaming::new_client(ip, |_| {}).unwrap(),
    // };

    let options = Default::default();
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp::new()))
        }),
    )
    .unwrap();
    println!("Finished");
}
