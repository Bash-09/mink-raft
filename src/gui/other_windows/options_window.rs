use std::ops::RangeInclusive;

use egui::{Id, ScrollArea};
use wgpu_app::utils::persistent_window::PersistentWindow;

use crate::WindowManagerType;

pub fn new_options_window() -> PersistentWindow<WindowManagerType> {
    PersistentWindow::new(Box::new(move |id, _, gui_ctx, state| {
        let mut open = true;

        egui::Window::new("Settings")
            .id(Id::new(id))
            .open(&mut open)
            .show(gui_ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.collapsing("Window", |ui| {
                        ui.label("No settings here yet");
                    });

                    ui.collapsing("Camera", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("FOV");
                            let mut fov = state.settings.fov;
                            if ui
                                .add(egui::Slider::new(
                                    &mut fov,
                                    RangeInclusive::new(60.0, 120.0),
                                ))
                                .changed()
                            {
                                // state.rend.cam.set_fov(fov);
                                tracing::error!("Need to set camera fov");
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Fog near");
                            ui.add(egui::DragValue::new(&mut state.settings.fog_near));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Fog far");
                            ui.add(egui::DragValue::new(&mut state.settings.fog_far));
                        });
                    });

                    ui.collapsing("Input", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Mouse sensitivity");
                            ui.add(egui::Slider::new(
                                &mut state.settings.mouse_sensitivity,
                                RangeInclusive::new(0.1, 10.0),
                            ));
                        });
                    });
                });
            });

        open
    }))
}
