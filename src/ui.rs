use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

use anyhow::Result;
use eframe::egui;

use crate::{
    config::{AppConfig, KnownDevice, Language, UpdateChannel},
    consent::{ConsentStatus, ConsentStore},
    events,
    identity::DeviceIdentity,
    session_history::{self, AccessAttemptStatus, SessionHistory},
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
                available_update: None,
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
    update_result: Option<Receiver<UpdateCheckResult>>,
    available_update: Option<update::AvailableUpdate>,
    status: Option<String>,
}

enum UpdateCheckResult {
    Available(update::AvailableUpdate),
    Current,
    Failed(String),
}

fn text<'a>(language: &Language, spanish: &'a str, english: &'a str) -> &'a str {
    match language {
        Language::Spanish => spanish,
        Language::English => english,
    }
}

fn consent_status(language: &Language, status: &ConsentStatus) -> &'static str {
    match (language, status) {
        (Language::Spanish, ConsentStatus::Pending) => "Pendiente",
        (Language::Spanish, ConsentStatus::Approved) => "Aprobada",
        (Language::Spanish, ConsentStatus::Denied) => "Rechazada",
        (Language::Spanish, ConsentStatus::Expired) => "Vencida",
        (Language::English, ConsentStatus::Pending) => "Pending",
        (Language::English, ConsentStatus::Approved) => "Approved",
        (Language::English, ConsentStatus::Denied) => "Denied",
        (Language::English, ConsentStatus::Expired) => "Expired",
    }
}

fn access_status(language: &Language, status: &AccessAttemptStatus) -> &'static str {
    match (language, status) {
        (Language::Spanish, AccessAttemptStatus::Requested) => "Solicitada",
        (Language::Spanish, AccessAttemptStatus::Approved) => "Aprobada",
        (Language::Spanish, AccessAttemptStatus::Denied) => "Rechazada",
        (Language::English, AccessAttemptStatus::Requested) => "Requested",
        (Language::English, AccessAttemptStatus::Approved) => "Approved",
        (Language::English, AccessAttemptStatus::Denied) => "Denied",
    }
}

impl AnyMioApp {
    fn save(&mut self, detail: &str) {
        match self.config.save(&self.data_dir) {
            Ok(()) => {
                let _ = events::append(&self.data_dir, "configuration_saved", detail);
                self.status = Some(
                    text(
                        &self.config.language,
                        "Configuración guardada.",
                        "Configuration saved.",
                    )
                    .into(),
                );
            }
            Err(error) => self.status = Some(format!("No se pudo guardar: {error}")),
        }
    }

    fn check_updates(&mut self) {
        let url = self.manifest_url.clone();
        let language = self.config.language.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = match tokio::runtime::Runtime::new() {
                Ok(runtime) => match runtime.block_on(update::check(&url)) {
                    Ok(Some(release)) => UpdateCheckResult::Available(release),
                    Ok(None) => UpdateCheckResult::Current,
                    Err(error) => UpdateCheckResult::Failed(error.to_string()),
                },
                Err(error) => UpdateCheckResult::Failed(format!(
                    "{}: {error}",
                    text(
                        &language,
                        "No se pudo iniciar la comprobación",
                        "Could not start the check",
                    )
                )),
            };
            let _ = sender.send(result);
        });
        self.available_update = None;
        self.status = Some(
            text(
                &self.config.language,
                "Buscando actualizaciones…",
                "Checking for updates…",
            )
            .into(),
        );
        self.update_result = Some(receiver);
    }

    fn resolve_consent(&mut self, id: uuid::Uuid, approved: bool) {
        match ConsentStore::load(&self.data_dir).and_then(|mut store| {
            let request = store.resolve(id, approved)?;
            store.save(&self.data_dir)?;
            Ok(request)
        }) {
            Ok(request) => {
                let action = if approved {
                    "consent_approved"
                } else {
                    "consent_denied"
                };
                match session_history::record(
                    &self.data_dir,
                    request.id,
                    request.requester_device_id,
                    if approved {
                        AccessAttemptStatus::Approved
                    } else {
                        AccessAttemptStatus::Denied
                    },
                ) {
                    Ok(()) => {
                        let _ = events::append(&self.data_dir, action, &id.to_string());
                        self.status = Some(if approved {
                            "Solicitud aprobada localmente.".into()
                        } else {
                            "Solicitud rechazada localmente.".into()
                        });
                    }
                    Err(error) => {
                        self.status = Some(format!(
                            "La solicitud se resolvió, pero no se pudo auditar: {error}"
                        ));
                    }
                }
            }
            Err(error) => self.status = Some(format!("No se pudo resolver la solicitud: {error}")),
        }
    }
}

impl eframe::App for AnyMioApp {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        let language = self.config.language.clone();
        if let Some(receiver) = &self.update_result {
            if let Ok(result) = receiver.try_recv() {
                let message = match result {
                    UpdateCheckResult::Available(release) => {
                        let version = release.version.clone();
                        self.available_update = Some(release);
                        format!(
                            "{} {version} {}",
                            text(&language, "Nueva versión", "New version"),
                            text(&language, "disponible.", "available.")
                        )
                    }
                    UpdateCheckResult::Current => text(
                        &language,
                        "Ya tienes la versión más reciente.",
                        "You already have the latest version.",
                    )
                    .into(),
                    UpdateCheckResult::Failed(error) => format!(
                        "{}: {error}",
                        text(&language, "No se pudo comprobar", "Could not check")
                    ),
                };
                let _ = events::append(&self.data_dir, "update_check_manual", &message);
                self.status = Some(message);
                self.update_result = None;
            } else {
                context.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        egui::TopBottomPanel::top("header").show(context, |ui| {
            ui.heading("AnyMio");
            ui.label(text(
                &language,
                "Control remoto personal — estado local visible",
                "Personal remote control — visible local status",
            ));
        });
        egui::CentralPanel::default().show(context, |ui| {
            ui.heading(text(&language, "Este equipo", "This device"));
            ui.monospace(format!("ID: {}", self.identity.public_id_formatted()));
            ui.label(format!(
                "{}: {}",
                text(&language, "Versión", "Version"),
                update::CURRENT_VERSION
            ));
            ui.separator();

            ui.heading(text(&language, "Actualizaciones", "Updates"));
            ui.checkbox(
                &mut self.config.check_updates_at_startup,
                text(
                    &language,
                    "Buscar actualizaciones al iniciar",
                    "Check for updates at startup",
                ),
            );
            egui::ComboBox::from_label(text(&language, "Canal", "Channel"))
                .selected_text(match &self.config.update_channel {
                    UpdateChannel::Stable => text(&language, "Estable", "Stable"),
                    UpdateChannel::Beta => "Beta",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.config.update_channel,
                        UpdateChannel::Stable,
                        text(&language, "Estable", "Stable"),
                    );
                    ui.selectable_value(&mut self.config.update_channel, UpdateChannel::Beta, "Beta");
                });
            egui::ComboBox::from_label(text(&language, "Idioma", "Language"))
                .selected_text(match &self.config.language {
                    Language::Spanish => "Español",
                    Language::English => "English",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.language, Language::Spanish, "Español");
                    ui.selectable_value(&mut self.config.language, Language::English, "English");
                });
            if ui
                .button(text(
                    &language,
                    "Buscar actualizaciones ahora",
                    "Check for updates now",
                ))
                .clicked()
                && self.update_result.is_none()
            {
                self.check_updates();
            }
            if let Some(release) = self.available_update.as_ref().cloned()
                && ui
                    .button(format!(
                        "{} {}",
                        text(&language, "Descargar e instalar", "Download and install"),
                        release.version
                    ))
                    .clicked()
            {
                match crate::start_update(&release) {
                    Ok(()) => {
                        let _ = events::append(
                            &self.data_dir,
                            "update_install_started",
                            &release.version,
                        );
                        context.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    Err(error) => {
                        self.status = Some(format!("No se pudo iniciar la actualización: {error}"));
                    }
                }
            }
            if let Some(notes) = self
                .available_update
                .as_ref()
                .and_then(|release| release.notes.as_deref())
            {
                ui.label(format!("{}: {notes}", text(&language, "Cambios", "Changes")));
            }
            if ui
                .button(text(&language, "Guardar preferencias", "Save preferences"))
                .clicked()
            {
                self.save("update channel, language or startup preference changed");
            }
            ui.separator();

            ui.heading("Relay");
            ui.horizontal(|ui| {
                ui.label("URL:");
                ui.text_edit_singleline(&mut self.relay_draft);
                if ui
                    .button(text(&language, "Guardar relay", "Save relay"))
                    .clicked()
                {
                    self.config.relay_url = (!self.relay_draft.trim().is_empty())
                        .then(|| self.relay_draft.trim().to_owned());
                    self.save("relay configuration changed");
                }
            });
            ui.separator();

            ui.heading(text(&language, "Dispositivos conocidos", "Known devices"));
            ui.horizontal(|ui| {
                ui.label(text(&language, "Nombre", "Name"));
                ui.text_edit_singleline(&mut self.device_name_draft);
                ui.label("ID");
                ui.text_edit_singleline(&mut self.device_id_draft);
                if ui
                    .button(text(&language, "Guardar dispositivo", "Save device"))
                    .clicked()
                {
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
            let mut forget_device = None;
            for device in &self.config.known_devices {
                ui.horizontal(|ui| {
                    ui.label(format!("{} — {}", device.display_name, device.public_id));
                    if ui.button(text(&language, "Olvidar", "Forget")).clicked() {
                        forget_device = Some(device.public_id.clone());
                    }
                });
            }
            if let Some(device_id) = forget_device {
                match self.config.remove_device(&device_id) {
                    Ok(()) => {
                        let _ = events::append(&self.data_dir, "known_device_forgotten", &device_id);
                        self.save("known device removed");
                    }
                    Err(error) => self.status = Some(format!("No se pudo eliminar dispositivo: {error}")),
                }
            }
            ui.separator();
            ui.heading(text(&language, "Consentimiento local", "Local consent"));
            ui.horizontal(|ui| {
                ui.label(text(&language, "ID solicitante", "Requester ID"));
                ui.text_edit_singleline(&mut self.consent_device_draft);
                if ui
                    .button(text(&language, "Crear solicitud", "Create request"))
                    .clicked()
                {
                    let requester = self.consent_device_draft.trim().to_owned();
                    match ConsentStore::load(&self.data_dir).and_then(|mut store| {
                        let id = store.request(requester.clone())?;
                        store.save(&self.data_dir)?;
                        session_history::record(
                            &self.data_dir,
                            id,
                            requester,
                            AccessAttemptStatus::Requested,
                        )?;
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
            let mut consent_action = None;
            if let Ok(store) = ConsentStore::load(&self.data_dir) {
                for request in store.requests.iter().rev().take(5) {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} — {}",
                            request.requester_device_id,
                            consent_status(&language, &request.status)
                        ));
                        if request.status == ConsentStatus::Pending {
                            if ui.button(text(&language, "Aprobar", "Approve")).clicked() {
                                consent_action = Some((request.id, true));
                            }
                            if ui.button(text(&language, "Rechazar", "Deny")).clicked() {
                                consent_action = Some((request.id, false));
                            }
                        }
                    });
                }
            }
            if let Some((id, approved)) = consent_action {
                self.resolve_consent(id, approved);
            }
            ui.separator();
            ui.heading(text(
                &language,
                "Historial de solicitudes",
                "Request history",
            ));
            match SessionHistory::load(&self.data_dir) {
                Ok(history) if history.entries.is_empty() => {
                    ui.label(text(
                        &language,
                        "Todavía no hay solicitudes registradas.",
                        "No requests have been recorded yet.",
                    ));
                }
                Ok(history) => {
                    for attempt in history.recent(5) {
                        ui.label(format!(
                            "{} — {}",
                            attempt.requester_device_id,
                            access_status(&language, &attempt.status)
                        ));
                    }
                }
                Err(error) => {
                    ui.label(format!("No se pudo leer el historial: {error}"));
                }
            }
            if let Some(status) = &self.status {
                ui.separator();
                ui.label(status);
            }
            ui.separator();
            ui.heading(text(&language, "Actividad reciente", "Recent activity"));
            match events::recent(&self.data_dir, 5) {
                Ok(events) if events.is_empty() => {
                    ui.label(text(
                        &language,
                        "Todavía no hay actividad registrada.",
                        "No activity has been recorded yet.",
                    ));
                }
                Ok(events) => { for event in events { ui.label(format!("{}: {}", event.kind, event.detail)); } }
                Err(error) => {
                    ui.label(format!(
                        "{}: {error}",
                        text(&language, "No se pudo leer la actividad", "Could not read activity")
                    ));
                }
            }
            ui.separator();
            ui.colored_label(
                egui::Color32::YELLOW,
                text(
                    &language,
                    "No hay sesión remota activa. El acceso remoto requiere consentimiento y cifrado E2E antes de habilitarse.",
                    "No remote session is active. Remote access requires consent and E2E encryption before it can be enabled.",
                ),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_statuses_follow_the_selected_language() {
        assert_eq!(text(&Language::English, "Guardar", "Save"), "Save");
        assert_eq!(
            consent_status(&Language::Spanish, &ConsentStatus::Pending),
            "Pendiente"
        );
        assert_eq!(
            access_status(&Language::English, &AccessAttemptStatus::Approved),
            "Approved"
        );
    }
}
