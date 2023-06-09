use std::collections::HashMap;

use eframe::{
    egui::{self, Id, LayerId, Order, Ui},
    epaint::{Color32, Hsva, Pos2, QuadraticBezierShape, Rgba, Shape, Stroke},
};

use super::rack::Rack;
use crate::{
    instance::{
        instance::{InstanceHandle, InstanceResponse},
        port::PortResponse,
    },
    io::ConnectResult,
};

pub struct RackResponse {
    responses: HashMap<InstanceHandle, InstanceResponse>,
}

impl RackResponse {
    pub fn new(responses: HashMap<InstanceHandle, InstanceResponse>) -> Self {
        Self { responses }
    }

    fn get_port(&self, mut predicate: impl FnMut(&PortResponse) -> bool) -> Option<&PortResponse> {
        self.responses
            .values()
            .flat_map(|response| response.ports.values().collect::<Vec<_>>())
            .find(|&port| predicate(port))
    }

    pub fn get_hovered_port(&self) -> Option<&PortResponse> {
        self.get_port(|port| port.hovered)
    }

    pub fn get_released_port(&self) -> Option<&PortResponse> {
        self.get_port(|port| port.released)
    }

    pub fn get_dragging_port(&self) -> Option<&PortResponse> {
        self.get_port(|port| port.dragging)
    }

    pub fn get_removed_instance(&self) -> Option<&InstanceResponse> {
        self.responses.values().find(|response| response.remove)
    }

    pub fn get_response(&self, handle: InstanceHandle) -> Option<&InstanceResponse> {
        self.responses.get(&handle)
    }

    pub fn show_connections(&self, rack: &Rack, ui: &mut Ui) {
        for (&from, connections) in rack.io.connections.iter() {
            for &to in connections.iter() {
                let from_response = self.get_response(from.instance).unwrap();
                let to_response = self.get_response(to.instance).unwrap();

                let from_port_response = from_response.get_port_response(from).unwrap();
                let to_port_response = to_response.get_port_response(to).unwrap();

                draw_rope(
                    from_port_response.position,
                    to_port_response.position,
                    ui,
                    Stroke::new(2.0, Hsva::new(0.0, 0.0, 1.0, 0.1)),
                );
            }
        }
    }

    pub fn show_dragged(&self, rack: &mut Rack, ui: &mut Ui) {
        if let Some(dragged) = self.get_dragging_port() {
            let can_connect = if let Some(hovered) = self.get_hovered_port() {
                let result = rack.can_connect(dragged.handle, hovered.handle);

                match result {
                    ConnectResult::Ok => {
                        egui::containers::show_tooltip_at_pointer(
                            ui.ctx(),
                            Id::new(hovered.description.id),
                            |ui| ui.label("✅connect"),
                        );
                    }
                    ConnectResult::Replace(..) | ConnectResult::Conversion => {
                        egui::containers::show_tooltip_at_pointer(
                            ui.ctx(),
                            Id::new(hovered.description.id),
                            |ui| ui.label(format!("⚠{}", result.as_str())),
                        );
                    }
                    _ => {
                        egui::containers::show_tooltip_at_pointer(
                            ui.ctx(),
                            Id::new(hovered.description.id),
                            |ui| ui.label(format!("❌{}", result.as_str())),
                        );
                    }
                }

                Some(result)
            } else {
                None
            };

            let stroke = if let Some(can_connect) = can_connect {
                match can_connect {
                    ConnectResult::Ok => Stroke::new(2.0, Color32::GREEN),
                    ConnectResult::Replace(..) | ConnectResult::Conversion => {
                        Stroke::new(2.0, Color32::GOLD)
                    }
                    _ => Stroke::new(2.0, Color32::RED),
                }
            } else {
                Stroke::new(2.0, Rgba::WHITE)
            };

            if let Some(mouse_pos) = ui.ctx().pointer_interact_pos() {
                draw_rope(dragged.position, mouse_pos, ui, stroke)
            }
        }
    }

    pub fn process(&self, rack: &mut Rack) {
        //connect when output connection drag released over other input connection
        if let Some(from) = self.get_released_port() {
            if let Some(to) = self.get_hovered_port() {
                rack.connect(from.handle, to.handle).ok();
            }
        }

        //remove removed
        if let Some(removed) = self.get_removed_instance() {
            rack.remove_instance(removed.handle)
        }
    }
}

pub fn draw_rope(from: Pos2, to: Pos2, ui: &mut Ui, stroke: Stroke) {
    let layer = LayerId::new(Order::Middle, Id::from("dragged"));
    let mut painter = ui.ctx().layer_painter(layer);
    let control = control_point(from, to);
    let shape = Shape::QuadraticBezier(QuadraticBezierShape {
        points: [from, control, to],
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke,
    });

    painter.set_clip_rect(ui.clip_rect());
    painter.add(shape);
}

fn control_point(a: Pos2, b: Pos2) -> Pos2 {
    let mut middle = (b - a) / 2.0;
    middle.y += a.distance(b) / 5.0;
    middle.y += (b.y - a.y).max(0.0) / 3.0;
    a + middle
}
