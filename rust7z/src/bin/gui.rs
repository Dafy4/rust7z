use eframe::egui;
use rfd::FileDialog;
use std::process::Command;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rust7z Compressor",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

struct App {
    input_file: String,
    output_file: String,
    algo: String,
    lossy: u8,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input_file: String::new(),
            output_file: String::new(),
            algo: "huffman".to_string(),
            lossy: 5,
            status: "Prêt".to_string(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Rust7z - Compresseur avec perte");

            ui.horizontal(|ui| {
                ui.label("Fichier source:");
                if ui.button("Parcourir").clicked() {
                    if let Some(path) = FileDialog::new().pick_file() {
                        self.input_file = path.display().to_string();
                    }
                }
                ui.label(&self.input_file);
            });

            ui.horizontal(|ui| {
                ui.label("Fichier de sortie:");
                if ui.button("Parcourir").clicked() {
                    if let Some(path) = FileDialog::new().save_file() {
                        self.output_file = path.display().to_string();
                    }
                }
                ui.label(&self.output_file);
            });

            ui.horizontal(|ui| {
                ui.label("Algorithme:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.algo)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.algo, "huffman".to_string(), "Huffman");
                        ui.selectable_value(&mut self.algo, "lz77".to_string(), "LZ77");
                        ui.selectable_value(&mut self.algo, "bwt".to_string(), "BWT");
                        ui.selectable_value(&mut self.algo, "auto".to_string(), "Auto");
                    });
            });

            ui.add(egui::Slider::new(&mut self.lossy, 0..=10).text("Niveau de perte"));

            if ui.button("Compresser").clicked() {
                if !self.input_file.is_empty() && !self.output_file.is_empty() {
                    self.status = "Compression en cours...".to_string();
                    let result = Command::new("rust7z")
                        .arg("compress")
                        .arg(&self.input_file)
                        .arg(&self.output_file)
                        .arg("--algo")
                        .arg(&self.algo)
                        .arg("--lossy")
                        .arg(self.lossy.to_string())
                        .output();
                    match result {
                        Ok(output) => {
                            if output.status.success() {
                                self.status = format!("Compression réussie ! {}", String::from_utf8_lossy(&output.stdout));
                            } else {
                                self.status = format!("Erreur : {}", String::from_utf8_lossy(&output.stderr));
                            }
                        }
                        Err(e) => self.status = format!("Erreur système : {}", e),
                    }
                } else {
                    self.status = "Veuillez sélectionner les fichiers".to_string();
                }
            }

            ui.label(&self.status);
        });
    }
}