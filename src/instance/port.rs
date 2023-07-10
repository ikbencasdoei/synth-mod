use eframe::{
    egui::{self, Button, Layout, Margin, RichText, Sense, Ui},
    emath::Align,
    epaint::{Color32, Hsva, Pos2, Shadow, Stroke, Vec2},
};

use super::instance::InstanceHandle;
use crate::{
    io::PortHandle,
    module::{PortDescriptionDyn, PortType},
    rack::rack::ShowContext,
};

/// Holds an instance of a [`crate::module::Port`]
pub struct PortInstance {
    pub description: PortDescriptionDyn,
    dragging: bool,
    pub handle: PortHandle,
}

impl PortInstance {
    pub fn from_description(description: &PortDescriptionDyn, instance: InstanceHandle) -> Self {
        Self {
            description: description.clone(),
            dragging: false,
            handle: PortHandle::new(description.id, instance),
        }
    }

    pub fn show(&mut self, ctx: &mut ShowContext, ui: &mut Ui) -> PortResponse {
        let mut response = PortResponse::new(self);

        let frame_response = egui::Frame::menu(ui.style())
            .shadow(Shadow::NONE)
            .outer_margin(Margin::same(2.0))
            .show(ui, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(self.description.name);
                    ui.label(RichText::new(self.description.type_name).color(Color32::LIGHT_BLUE));

                    if let PortType::Input = self.description.port_type {
                        if !ctx.has_connection(self.handle) {
                            self.description
                                .closure_edit
                                .as_ref()
                                .expect("this closure should be available on input ports")(
                                self.handle,
                                ctx,
                                ui,
                            )
                        }
                    }

                    self.show_port_visual(&mut response, ctx, ui);

                    if ctx.has_connection(self.handle)
                        && ui.add(Button::new("❌").small()).clicked()
                    {
                        ctx.clear_port(self.handle)
                    }
                });
            });

        if let PortType::Input = self.description.port_type {
            if frame_response.response.hovered() {
                response.hovered = true;
            }
        }

        if !ui.memory(|memory| memory.is_anything_being_dragged()) && self.dragging {
            self.dragging = false;
            response.released = true;
        }

        response.dragging = self.dragging;

        response
    }

    fn show_port_visual(
        &mut self,
        response: &mut PortResponse,
        ctx: &mut ShowContext,
        ui: &mut Ui,
    ) {
        let sense = if let PortType::Output = self.description.port_type {
            Sense::drag()
        } else {
            Sense::hover()
        };

        let desired_size = ui.spacing().interact_size.y * Vec2::splat(1.0);
        let (rect, port_response) = ui.allocate_exact_size(desired_size, sense);

        if port_response.drag_started() {
            self.dragging = true;
        }

        response.position = rect.center();

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&port_response);
            let rect = rect.expand(visuals.expansion);
            let radius = 0.5 * rect.height();
            let inner_radius = 0.5 * radius;
            let stroke = Stroke::new(visuals.fg_stroke.width + 0.5, visuals.fg_stroke.color);

            ui.painter()
                .circle(rect.center(), radius, visuals.bg_fill, stroke);

            match self.description.port_type {
                PortType::Input => {
                    ui.painter()
                        .circle(rect.center(), inner_radius, visuals.bg_fill, stroke);

                    let value: f32 = if ctx.has_connection(self.handle) {
                        if let Some(boxed) = ctx.get_input_boxed(self.handle) {
                            boxed.as_value()
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };

                    ui.painter().circle_filled(
                        rect.center(),
                        0.5 * inner_radius,
                        Hsva::new(0.5, 1.0, value.clamp(0.0, 1.0), 1.0),
                    );
                }
                PortType::Output => {
                    if !ctx.has_connection(self.handle) {
                        ui.painter().circle_filled(
                            rect.center(),
                            inner_radius,
                            visuals.fg_stroke.color,
                        );
                    } else {
                        ui.painter()
                            .circle_filled(rect.center(), inner_radius, Color32::WHITE);
                    }
                }
            }
        }

        if let PortType::Input = self.description.port_type {
            if !ctx.has_connection(self.handle) {
                port_response.on_hover_text_at_pointer("Input");
            } else {
                port_response.on_hover_text_at_pointer(self
                    .description
                    .closure_value
                    .as_ref()
                    .expect("this closure should be available on input ports")(
                    self.handle, ctx
                ));
            }
        }
    }
}

/// Contains useful data after drawing an instance's ui.
pub struct PortResponse {
    pub description: PortDescriptionDyn,
    pub position: Pos2,
    pub dragging: bool,
    pub released: bool,
    pub hovered: bool,
    pub handle: PortHandle,
}

impl PortResponse {
    fn new(port: &PortInstance) -> Self {
        Self {
            description: port.description.clone(),
            position: Pos2::ZERO,
            dragging: false,
            released: false,
            hovered: false,
            handle: port.handle,
        }
    }
}
