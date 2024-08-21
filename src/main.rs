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

#[derive(Clone, Copy, PartialEq)]  // Aggiunto PartialEq per l'enum Mode
enum Mode {
    Caster,
    Receiver,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Caster
    }
}

enum TransmissionStatus {
    Idle,
    Casting,
    Receiving,
}

impl Default for TransmissionStatus {
    fn default() -> Self {
        TransmissionStatus::Idle
    }
}

struct ScreenArea {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

struct MyApp {
    _streaming: Streaming,
    current_image: Arc<Mutex<Option<egui::ColorImage>>>,
    texture: Option<egui::TextureHandle>,
    mode: Mode,
    caster_address: String,
    selected_screen_area: Option<ScreenArea>,
    transmission_status: TransmissionStatus,
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
            let image = egui::ColorImage::from_rgba_premultiplied(size, &image);

            // println!("Received image with size {:?}", size);

            *image_clone.lock().unwrap() = Some(image);
        })
        .unwrap();
        streaming.start().unwrap();
        Self {
            _streaming: streaming,
            current_image,
            texture: None,
            mode: Mode::default(),
            caster_address: String::default(),
            selected_screen_area: None,
            transmission_status: TransmissionStatus::default()
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Screen-Caster");

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.radio_value(&mut self.mode, Mode::Caster, "Caster");
                ui.radio_value(&mut self.mode, Mode::Receiver, "Receiver");
            });

            ui.separator();

            match self.mode {
                Mode::Caster => {
                    ui.label("Select screen area:");
                    ui.separator();

                    // Mock screen area selection (replace with actual logic)
                    ui.add_space(8.0);

                    let mut data = self.current_image.lock().unwrap();
                    if let Some(image) = data.take() {
                        self.texture = Some(ui.ctx().load_texture("image", image, Default::default()));
                    }
                    drop(data);

                    if let Some(texture) = &self.texture {
                        ui.add(egui::Image::from_texture(texture).shrink_to_fit());
                    }

                }
                Mode::Receiver => {
                    ui.label("Enter caster's address:");
                    // Input field for the address (replace with actual logic)
                    ui.text_edit_singleline(&mut self.caster_address);
                }
            }

            ui.separator();

            match self.transmission_status {
                TransmissionStatus::Idle => {
                    match self.mode {
                        Mode::Caster => {
                            if ui.button("Start trasmission").clicked() {
                                self.transmission_status = TransmissionStatus::Casting;
                            }
                        }
                        Mode::Receiver => {
                            if ui.button("Start reception").clicked() {
                                self.transmission_status = TransmissionStatus::Receiving;
                            }
                        }
                    }
                }
                TransmissionStatus::Casting => {
                    ui.label("Casting...");
                    if ui.button("Stop transmission").clicked() {
                        self.transmission_status = TransmissionStatus::Idle;
                    }
                }
                TransmissionStatus::Receiving => {
                    ui.label("Receiving...");
                    if ui.button("Stop reception").clicked() {
                        self.transmission_status = TransmissionStatus::Idle;
                    }
                }
            }
        });
    }

    /*fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
    }*/
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
