use egui::{Align2, Context, Id, ScrollArea, Vec2};
use wgpu_app::utils::persistent_window::PersistentWindow;

use crate::{
    network::{NetworkCommand, NetworkManager, PROTOCOL},
    server::Server,
    settings::SavedServer,
    App,
};

#[allow(clippy::too_many_lines)]
pub fn render(gui_ctx: &Context, cli: &mut App) -> Option<Server> {
    let mut serv = None;

    egui::SidePanel::left("Server manager")
        .resizable(true)
        .show(gui_ctx, |ui| {
            ui.heading("Account Settings");

            ui.radio_value(&mut cli.settings.online_play, true, "Online mode");
            ui.radio_value(&mut cli.settings.online_play, false, "Offline mode");

            ui.separator();

            if cli.settings.online_play {
                ui.label("Online play is not yet implemented");
            } else {
                ui.horizontal(|ui| {
                    ui.label("Player Name: ");
                    ui.text_edit_singleline(&mut cli.settings.name);
                });
            }
        });

    egui::CentralPanel::default().show(gui_ctx, |ui| {
        ui.heading("Servers");
        ui.add_space(15.0);

        ui.label("IP Address: ");
        ui.text_edit_singleline(&mut cli.settings.direct_connection);

        ui.horizontal(|ui| {
            if ui.button("Direct Connect").clicked() {
                match connect(&cli.settings.direct_connection, cli.settings.name.clone()) {
                    Ok(s) => serv = Some(s),
                    Err(e) => tracing::error!("Failed to connect to server: {:?}", e),
                }
            }

            if ui.button("Save Server").clicked() {
                let host = cli.settings.direct_connection.clone();
                let name = format!("Saved Server {}", cli.settings.saved_servers.len() + 1);
                cli.settings
                    .saved_servers
                    .push(SavedServer { ip: host, name });
            }
        });
        ui.separator();

        ScrollArea::vertical().show(ui, |ui| {
            let App {
                settings,
                server_pings,
                outstanding_server_pings,
                // icon_handles,
                ..
            } = cli;
            let wm = &mut cli.window_manager;

            let mut remove: Option<usize> = None;
            for (i, s) in settings.saved_servers.iter().enumerate() {
                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    ui.add_space(15.0);

                    // Info and controls
                    ui.vertical(|ui| {
                        // Name and IP
                        ui.label(&s.name);
                        ui.label(&s.ip);

                        // Buttons
                        ui.horizontal(|ui| {
                            if ui.button("Connect").clicked() {
                                match connect(&s.ip, settings.name.clone()) {
                                    Ok(s) => serv = Some(s),
                                    Err(e) => {
                                        tracing::error!("Failed to connect to server: {:?}", e)
                                    }
                                }
                            }
                            if ui.button("Refresh").clicked() {
                                tracing::info!("Attempting to connect");
                                match NetworkManager::connect(&s.ip) {
                                    Ok(server) => {
                                        server.send_command(NetworkCommand::RequestStatus);
                                        outstanding_server_pings.insert(s.ip.clone(), server);
                                    }
                                    Err(e) => {
                                        tracing::error!("Couldn't get status from server: {:?}", e);
                                    }
                                };
                            }
                            if ui.button("Edit").clicked() {
                                let len = settings.saved_servers.len();

                                let index = i;
                                let mut new = s.clone();

                                // Edit
                                wm.push(PersistentWindow::new(Box::new(
                                    move |id, _, gui_ctx, state| {
                                        let current_length = state.settings.saved_servers.len();
                                        if current_length != len || index >= current_length {
                                            return false;
                                        }
                                        let mut open = true;

                                        egui::Window::new("Modify server")
                                            .id(Id::new(id))
                                            .resizable(false)
                                            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                                            .collapsible(false)
                                            .show(gui_ctx, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label("Name:    ");
                                                    ui.text_edit_singleline(&mut new.name);
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("Address: ");
                                                    ui.text_edit_singleline(&mut new.ip);
                                                });

                                                ui.horizontal(|ui| {
                                                    if ui.button("Confirm").clicked() {
                                                        state.settings.saved_servers[index] =
                                                            new.clone();

                                                        open = false;
                                                    }

                                                    if ui.button("Cancel").clicked() {
                                                        open = false;
                                                    }
                                                });
                                            });
                                        open
                                    },
                                )));
                            }
                            if ui.button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    });

                    // Status info
                    ui.separator();
                    match server_pings.get(&s.ip) {
                        Some(status) => {
                            // Favicon
                            // if let Some(favicon) = &status.favicon {
                            // if icon_handles.get(&s.ip).is_none() {
                            //     // Load image
                            //     icon_handles.insert(
                            //         s.ip.clone(),
                            //         RetainedImage::from_image_bytes(
                            //             s.ip.clone(),
                            //             &favicon.data,
                            //         )
                            //         .unwrap(),
                            //     );
                            // }

                            // if let Some(icon) = icon_handles.get(&s.ip) {
                            //     // ui.image(tex_handle, Vec2::new(50.0, 50.0));

                            //     icon.show_size(ui, Vec2::new(50.0, 50.0));
                            // }
                            // }

                            // Version, Players, Ping
                            ui.vertical(|ui| {
                                if let Some(version) = &status.version {
                                    ui.label(&version.name);
                                }

                                let players = ui.label(&format!(
                                    "Players: {} / {}",
                                    status.players.online, status.players.max
                                ));
                                if status.players.online > 0 {
                                    players.on_hover_ui(|ui| {
                                        for p in &status.players.sample {
                                            ui.label(&p.name);
                                        }
                                    });
                                }
                                // ui.label(&format!("Ping: {}ms", status.ping));
                            });

                            if let Some(desc) = status.description.to_traditional() {
                                ui.label(&desc);
                            }
                        }
                        None => {}
                    }
                });

                ui.add_space(15.0);
                ui.separator();
            }

            if let Some(i) = remove {
                cli.settings.saved_servers.remove(i);
            }
        });
    });

    serv
}

fn connect(ip: &str, name: String) -> Result<Server, std::io::Error> {
    match NetworkManager::connect(ip) {
        Ok(server) => {
            tracing::debug!("Connected to server.");
            server.send_command(NetworkCommand::Login(PROTOCOL, 25565, name));

            Ok(server)
        }
        Err(e) => Err(e),
    }
}
