use std::sync::{Arc, Mutex};

use image::ImageFormat;

use eframe::egui::{self, Color32, Key};

use std::net::Ipv4Addr;

use crate::streaming::Streaming;
use winit::event_loop::EventLoop;

fn is_valid_ipv4(ip: &str) -> bool {
    ip.parse::<Ipv4Addr>().is_ok()
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

#[derive(PartialEq)]
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

#[derive(PartialEq, Clone)]
struct ScreenArea {
    startx: u32,
    starty: u32,
    endx: u32,
    endy: u32,
}

pub struct MyApp {
    _streaming: Option<Streaming>,
    current_image: Arc<Mutex<Option<egui::ColorImage>>>,
    texture: Option<egui::TextureHandle>,
    mode: Mode,
    caster_address: String,
    selected_screen_area: Option<ScreenArea>,
    transmission_status: TransmissionStatus,
    pause: bool,
    error_msg: Option<String>,
    blanking_screen: bool,
    slider_value1: f32,
    slider_value2: f32,
    slider_value3: f32,
    slider_value4: f32,
    screen_width: u32,
    screen_height: u32,

}

impl MyApp {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();

        // Get monitor dimensions
        let primary_monitor = event_loop.available_monitors().next().expect("No monitors available");
        let video_mode = primary_monitor
            .video_modes()
            .next()
            .expect("No video modes available");
        let screen_width = video_mode.size().width;
        let screen_height = video_mode.size().height;



        let current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
            [200, 200],
            Color32::BLACK,
        ))));
        
        

        Self {
            _streaming: None,
            current_image,
            texture: None,
            mode: Mode::default(),
            caster_address: String::default(),
            selected_screen_area: None,
            transmission_status: TransmissionStatus::default(),
            pause: false,
            error_msg: None,
            blanking_screen: false,
            slider_value1: 0.0,
            slider_value2: 0.0,
            slider_value3: 0.0,
            slider_value4: 0.0,
            screen_width: screen_width,
            screen_height: screen_height,

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
                ui.add_enabled_ui(self.transmission_status == TransmissionStatus::Idle, |ui| {
                    if ui.radio(self.mode == Mode::Caster, "Caster").clicked() {
                        self.error_msg.take();
                        self.mode = Mode::Caster;
                    }
                });
                ui.add_enabled_ui(self.transmission_status == TransmissionStatus::Idle, |ui| {
                    if ui.radio(self.mode == Mode::Receiver, "Receiver").clicked() {
                        self.error_msg.take();
                        self.mode = Mode::Receiver;
                    }
                });
            });

            match &self.error_msg {
                Some(msg) => {
                    ui.colored_label(egui::Color32::RED, msg);
                }
                _ => {}
            }

            ui.separator();

            match self.mode {
                Mode::Caster => {
                    ui.label("Select screen area:");
                    ui.horizontal(|ui| {
                        if ui.selectable_value(&mut None, self.selected_screen_area.clone(), "Total screen").clicked(){
                            self.selected_screen_area = None;
                            self.slider_value1 = 0.0;
                            self.slider_value2 = 0.0;
                            self.slider_value3 = 0.0;
                            self.slider_value4 = 0.0;
                            if let Some(s) = &self._streaming {
                                if let Streaming::Server(ss) = &s{
                                    ss.capture_fullscreen();
                                }
                            }
                        }
                        if ui.selectable_value(&mut true, self.selected_screen_area.is_some(), "Personalized area").clicked(){
                            self.selected_screen_area = Some(ScreenArea {
                                startx: 0,
                                starty: 0,
                                endx: 0,
                                endy: 0,
                            });
                        }
                        if self.selected_screen_area.is_some() {
                            let mut maxs1 = self.screen_width-1;
                            let mut maxs3 = self.screen_width-1;
                            let mut maxs2: u32 = self.screen_height-1;
                            let mut maxs4 = self.screen_height-1;

                            if self.slider_value1 != 0.0 {
                                maxs3 = self.screen_width-1-(self.slider_value1 as u32);
                            };
                            if self.slider_value3 != 0.0 {
                                maxs1 = self.screen_width-1 -(self.slider_value3 as u32);
                            };
                            if self.slider_value2 != 0.0 {
                                maxs4 = self.screen_height-1 -(self.slider_value2 as u32);
                            };
                            if self.slider_value4 != 0.0 {
                                maxs2 = self.screen_height-1 -(self.slider_value4 as u32);
                                
                            };
                            ui.vertical_centered_justified(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Left:  ");
                                    ui.add(egui::Slider::new(&mut self.slider_value1, 0.0..= maxs1 as f32));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Right:");
                                    ui.add(egui::Slider::new(&mut self.slider_value3, 0.0..= maxs3 as f32));
                                });
                            });
                            ui.vertical_centered_justified(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Top:      ");
                                    ui.add(egui::Slider::new(&mut self.slider_value2, 0.0..= maxs2 as f32));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Bottom:");
                                    ui.add(egui::Slider::new(&mut self.slider_value4, 0.0..=maxs4 as f32));
                                });
                            });
                            if let Some(Streaming::Server(ss)) = &self._streaming {
                                let startx = self.slider_value1.round() as u32;
                                let starty = self.slider_value2.round() as u32;
                                let endx = self.screen_width - self.slider_value3.round() as u32;
                                let endy = self.screen_height - self.slider_value4.round() as u32;

                                #[cfg(any(target_os = "linux", target_os = "windows"))]
                                ss.capture_resize(startx, starty, endx, endy);
                                #[cfg(target_os = "macos")]
                                ss.capture_resize(
                                    self.slider_value1.round() as u32,
                                    self.slider_value2.round() as u32,
                                    self.slider_value3.round() as u32, 
                                    self.slider_value4.round() as u32,
                                );
                            }
                        };
                        if !self.selected_screen_area.is_some() {
                            if let Some(Streaming::Server(ss)) = &self._streaming {
                                ss.capture_fullscreen();
                            }
                        }
                    });
                }
                Mode::Receiver => {
                    ui.label("Enter caster's address:");

                    ui.add_enabled(self.transmission_status == TransmissionStatus::Idle, |ui: &mut egui::Ui|{
                        ui.text_edit_singleline(&mut self.caster_address)
                    });
                }
            }

            ui.separator();

            match &self.transmission_status {
                TransmissionStatus::Idle => {
                    match self.mode {
                        Mode::Caster => {
                            if ui.button("Start trasmission").clicked() {
                                if let Some(s) = &self._streaming{
                                    match s {
                                        Streaming::Client(_) => {
                                            let image_clone = self.current_image.clone();
                                            match Streaming::new_server(move |bytes| {
                                                let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                    .unwrap()
                                                    .to_rgba8();
                        
                                                let size = [image.width() as usize, image.height() as usize];
                                                let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                        
                                                *image_clone.lock().unwrap() = Some(image);
                                            }) {
                                                Ok(s) => {
                                                    self._streaming = Some(s);
                                                }
                                                Err(e) => {
                                                    self.error_msg = Some(e.to_string());
                                                }
                                            }
                                        }
                                        Streaming::Server(_) => { /* Nothing to do because it is already a streaming server */ }
                                    }
            
                                }
                                else{
                                    let image_clone = self.current_image.clone();
                                    match Streaming::new_server(move |bytes| {
                                        let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                            .unwrap()
                                            .to_rgba8();
                
                                        let size = [image.width() as usize, image.height() as usize];
                                        let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                
                                        *image_clone.lock().unwrap() = Some(image);
                                    }) {
                                        Ok(s) => {
                                            self._streaming = Some(s);
                                        }
                                        Err(e) => {
                                            self.error_msg = Some(e.to_string());
                                        }
                                    }
                                }
                                if let Some(s) = &self._streaming{
                                    self.pause = false;
                                    self.blanking_screen = false;
                                    self.error_msg.take();
                                    match s.start(){
                                        Ok(_) => {
                                            self.transmission_status = TransmissionStatus::Casting;
                                        }
                                        Err(e) => {
                                            self.error_msg = Some(e.to_string());
                                        }
                                    }
                                    
                                }
                            }
                        }
                        Mode::Receiver => {
                            ui.horizontal(|ui| {
                                if ui.button("Start reception without recording").clicked() {
                                    if is_valid_ipv4(&self.caster_address){
                                        let image_clone = self.current_image.clone();
                                        match Streaming::new_client(&self.caster_address, move |bytes| {
                                            let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                .unwrap()
                                                .to_rgba8();
                    
                                            let size = [image.width() as usize, image.height() as usize];
                                            let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                    
                                            *image_clone.lock().unwrap() = Some(image);
                                        }, false) {
                                            Ok(s) => {
                                                self._streaming = Some(s);
                                            }
                                            Err(e) => {
                                                self.error_msg = Some(e.to_string());
                                            }
                                        }
                                        if let Some(s) = &self._streaming{
                                            self.error_msg.take();
                                            match s.start(){
                                                Ok(_) => {
                                                    self.transmission_status = TransmissionStatus::Receiving;
                                                }
                                                Err(e) => {
                                                    self.error_msg = Some(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                    else{
                                        self.error_msg = Some("Please insert a valid IP address!".to_string());
                                    }
                                }
                                if ui.button("Start reception and save recording").clicked() {
                                    if is_valid_ipv4(&self.caster_address){
                                        let image_clone = self.current_image.clone();
                                        match Streaming::new_client(&self.caster_address, move |bytes| {
                                            let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                .unwrap()
                                                .to_rgba8();
                    
                                            let size = [image.width() as usize, image.height() as usize];
                                            let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                    
                                            *image_clone.lock().unwrap() = Some(image);
                                        },true) {
                                            Ok(s) => {
                                                self._streaming = Some(s);
                                            }
                                            Err(e) => {
                                                self.error_msg = Some(e.to_string());
                                            }
                                        }
                                        if let Some(s) = &self._streaming{
                                            self.error_msg.take();
                                            match s.start(){
                                                Ok(_) => {
                                                    self.transmission_status = TransmissionStatus::Receiving;
                                                }
                                                Err(e) => {
                                                    self.error_msg = Some(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                    else{
                                        self.error_msg = Some("Please insert a valid IP address!".to_string());
                                    }
                                }
                            });
                        }
                    }
                }
                TransmissionStatus::Casting => {
                    let input = ctx.input(|i| i.clone());
                    if !self.pause{
                        ui.label("Casting...");
                    }
                    else{
                        ui.colored_label(egui::Color32::LIGHT_RED, "Pause...");
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Stop transmission").on_hover_text("Ctrl + T").clicked() || input.key_pressed(Key::T) && input.modifiers.ctrl{
                            self._streaming.take();
                            self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                                [200, 200],
                                Color32::BLACK))));
                            self.transmission_status = TransmissionStatus::Idle;
                        }

                        if ui.add_enabled(!self.pause, egui::Button::new("Pause")).on_hover_text("Ctrl + P").clicked() || input.key_pressed(Key::P) && input.modifiers.ctrl{
                            self.pause = true;
                            if let Some(Streaming::Server(s)) = &self._streaming{
                                match s.pause(){
                                    Ok(_) => {}
                                    Err(e) => {
                                        self.error_msg = Some(e.to_string());
                                        self._streaming.take();
                                        self.transmission_status = TransmissionStatus::Idle;
                                    }
                                }
                            }
                        }
                        if ui.add_enabled(self.pause, egui::Button::new("Resume")).on_hover_text("Ctrl + R").clicked() || input.key_pressed(Key::R) && input.modifiers.ctrl{
                            self.pause = false;
                            if let Some(Streaming::Server(s)) = &self._streaming{
                                match s.start(){
                                    Ok(_) => {}
                                    Err(e) => {
                                        self.error_msg = Some(e.to_string());
                                        self._streaming.take();
                                        self.transmission_status = TransmissionStatus::Idle;
                                    }
                                }
                            }
                        }
                        if ui.selectable_value(&mut self.blanking_screen.clone(), true, "Blanking screen").on_hover_text("Ctrl + B").clicked() || input.key_pressed(Key::B) && input.modifiers.ctrl {
                            self.blanking_screen = !self.blanking_screen;
                            if let Some(Streaming::Server(s)) = &self._streaming {
                                if self.blanking_screen {
                                    s.blank_screen();
                                } else {
                                    s.restore_screen();
                                }
                            }
                        }
                    });

                }
                TransmissionStatus::Receiving => {
                    ui.label(format!("Receiving..."));
                    if ui.button("Stop reception").clicked() {
                        self._streaming.take();
                        self.caster_address = String::default();
                        self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                            [200, 200],
                            Color32::BLACK))));
                        self.transmission_status = TransmissionStatus::Idle;
                    }
                    if let Some(Streaming::Client(s)) = &self._streaming {
                        if !s.is_connected() {
                            self._streaming.take();
                            self.caster_address = String::default();
                            self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                                [200, 200],
                                Color32::BLACK))));
                            self.transmission_status = TransmissionStatus::Idle;
                        }
                    }
                }
            }

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