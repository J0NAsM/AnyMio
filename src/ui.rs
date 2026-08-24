use anyhow::Result;
use eframe::egui;

use crate::{config::AppConfig, identity::DeviceIdentity, update};

pub fn run(identity: DeviceIdentity, config: AppConfig) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([760.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "AnyMio",
        options,
        Box::new(|_| Ok(Box::new(AnyMioApp { identity, config }))),
    )
    .map_err(|error| anyhow::anyhow!("could not open the AnyMio window: {error}"))
}

struct AnyMioApp {
    identity: DeviceIdentity,
    config: AppConfig,
}

impl eframe::App for AnyMioApp {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(context, |ui| {
            ui.heading("AnyMio");
            ui.label("Control remoto personal — estado local visible");
        });
        egui::CentralPanel::default().show(context, |ui| {
            ui.heading("Este equipo");
            ui.monospace(format!("ID: {}", self.identity.public_id_formatted()));
            ui.label(format!("Versión: {}", update::CURRENT_VERSION));
            ui.separator();

            ui.heading("Actualizaciones");
            ui.label(format!("Canal: {:?}", self.config.update_channel));
            ui.label(if self.config.check_updates_at_startup {
                "Se comprobarán al iniciar."
            } else {
                "La comprobación automática está desactivada."
            });
            ui.label("Usa el comando --install-update para instalar una versión verificada.");
            ui.separator();

            ui.heading("Relay y dispositivos");
            ui.label(self.config.relay_url.as_deref().unwrap_or("Relay no configurado."));
            if self.config.known_devices.is_empty() {
                ui.label("No hay dispositivos conocidos todavía.");
            } else {
                for device in &self.config.known_devices {
                    ui.label(format!("{} — {}", device.display_name, device.public_id));
                }
            }
            ui.separator();
            ui.colored_label(
                egui::Color32::YELLOW,
                "No hay ninguna sesión remota activa. El acceso remoto requiere consentimiento y cifrado E2E antes de habilitarse.",
            );
        });
    }
}
