use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

use anyhow::Result;
use eframe::egui;

use crate::{
    config::{AppConfig, KnownDevice, Language, UpdateChannel},
    consent::ConsentStore,
    events,
    identity::DeviceIdentity,
    update,
};

pub fn run(
    identity: DeviceIdentity,
    config: AppConfig,
    data_dir: PathBuf,
    manifest_url: String,
) -> Result<()> {
    let relay_draft = config.relay_url.clone().unwrap_or_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([760.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "AnyMio",
        options,
        Box::new(|_| {
            Ok(Box::new(AnyMioApp {
                identity,
                config,
                data_dir,
                relay_draft,
                device_id_draft: String::new(),
                device_name_draft: String::new(),
                consent_device_draft: String::new(),
                manifest_url,
                update_result: None,
                status: None,
            }))
        }),
    )
    .map_err(|error| anyhow::anyhow!("could not open the AnyMio window: {error}"))
}

struct AnyMioApp {
    identity: DeviceIdentity,
    config: AppConfig,
    data_dir: PathBuf,
    relay_draft: String,
    device_id_draft: String,
    device_name_draft: String,
    consent_device_draft: String,
    manifest_url: String,
    update_result: Option<Receiver<String>>,
    status: Option<String>,
}

impl AnyMioApp {
    fn save(&mut self, detail: &str) {
        match self.config.save(&self.data_dir) {
            Ok(()) => {
                let _ = events::append(&self.data_dir, "configuration_saved", detail);
                self.status = Some("Configuración guardada.".into());
            }
            Err(error) => self.status = Some(format!("No se pudo guardar: {error}")),
        }
    }

    fn check_updates(&mut self) {
        let url = self.manifest_url.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let message = match tokio::runtime::Runtime::new() {
                Ok(runtime) => match runtime.block_on(update::check(&url)) {
                    Ok(Some(release)) => format!("Nueva versión {} disponible.", release.version),
                    Ok(None) => "Ya tienes la versión más reciente.".into(),
                    Err(error) => format!("No se pudo comprobar: {error}"),
                },
                Err(error) => format!("No se pudo iniciar la comprobación: {error}"),
            };
            let _ = sender.send(message);
        });
        self.status = Some("Buscando actualizaciones…".into());
        self.update_result = Some(receiver);
    }
}

impl eframe::App for AnyMioApp {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        if let Some(receiver) = &self.update_result {
            if let Ok(message) = receiver.try_recv() {
                let _ = events::append(&self.data_dir, "update_check_manual", &message);
                self.status = Some(message);
                self.update_result = None;
            } else {
                context.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
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
            ui.checkbox(
                &mut self.config.check_updates_at_startup,
                "Buscar actualizaciones al iniciar",
            );
            egui::ComboBox::from_label("Canal")
                .selected_text(format!("{:?}", self.config.update_channel))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.update_channel, UpdateChannel::Stable, "Estable");
                    ui.selectable_value(&mut self.config.update_channel, UpdateChannel::Beta, "Beta");
                });
            egui::ComboBox::from_label("Idioma")
                .selected_text(format!("{:?}", self.config.language))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.language, Language::Spanish, "Español");
                    ui.selectable_value(&mut self.config.language, Language::English, "English");
                });
            ui.label("Las instalaciones de versiones verificadas se realizan con --install-update.");
            if ui.button("Buscar actualizaciones ahora").clicked() && self.update_result.is_none() {
                self.check_updates();
            }
            ui.separator();

            ui.heading("Relay");
            ui.horizontal(|ui| {
                ui.label("URL:");
                ui.text_edit_singleline(&mut self.relay_draft);
                if ui.button("Guardar relay").clicked() {
                    self.config.relay_url = (!self.relay_draft.trim().is_empty())
                        .then(|| self.relay_draft.trim().to_owned());
                    self.save("relay configuration changed");
                }
            });
            ui.separator();

            ui.heading("Dispositivos conocidos");
            ui.horizontal(|ui| {
                ui.label("Nombre");
                ui.text_edit_singleline(&mut self.device_name_draft);
                ui.label("ID");
                ui.text_edit_singleline(&mut self.device_id_draft);
                if ui.button("Guardar dispositivo").clicked() {
                    let result = self.config.add_or_update_device(KnownDevice {
                        public_id: self.device_id_draft.trim().to_owned(),
                        display_name: self.device_name_draft.trim().to_owned(),
                        public_key_fingerprint: None,
                        last_seen_unix: None,
                    });
                    match result {
                        Ok(()) => {
                            self.device_id_draft.clear();
                            self.device_name_draft.clear();
                            self.save("known device saved");
                        }
                        Err(error) => self.status = Some(format!("Dispositivo inválido: {error}")),
                    }
                }
            });
            for device in &self.config.known_devices {
                ui.label(format!("{} — {}", device.display_name, device.public_id));
            }
            ui.separator();
            ui.heading("Consentimiento local");
            ui.horizontal(|ui| {
                ui.label("ID solicitante");
                ui.text_edit_singleline(&mut self.consent_device_draft);
                if ui.button("Crear solicitud").clicked() {
                    match ConsentStore::load(&self.data_dir).and_then(|mut store| {
                        let id = store.request(self.consent_device_draft.trim().to_owned())?;
                        store.save(&self.data_dir)?;
                        Ok(id)
                    }) {
                        Ok(id) => {
                            let _ = events::append(&self.data_dir, "consent_requested", &id.to_string());
                            self.consent_device_draft.clear();
                            self.status = Some(format!("Solicitud creada: {id}"));
                        }
                        Err(error) => self.status = Some(format!("No se pudo crear la solicitud: {error}")),
                    }
                }
            });
            if let Ok(store) = ConsentStore::load(&self.data_dir) {
                for request in store.requests.iter().rev().take(5) {
                    ui.label(format!("{} — {:?}", request.requester_device_id, request.status));
                }
            }
            if let Some(status) = &self.status {
                ui.separator();
                ui.label(status);
            }
            ui.separator();
            ui.colored_label(
                egui::Color32::YELLOW,
                "No hay sesión remota activa. El acceso remoto requiere consentimiento y cifrado E2E antes de habilitarse.",
            );
        });
    }
}
