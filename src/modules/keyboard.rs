use eframe::{
    egui::{self, style::Widgets, Layout, Ui},
    epaint::{Color32, Hsva, Vec2},
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    module::{Module, ModuleDescription, Port},
    rack::rack::{ProcessContext, ShowContext},
};

#[derive(Clone, Copy, EnumIter)]
enum Tone {
    C,
    Cs,
    D,
    Ds,
    E,
    F,
    Fs,
    G,
    Gs,
    A,
    As,
    B,
}

impl Tone {
    fn is_sharp(&self) -> bool {
        match self {
            Tone::Cs => true,
            Tone::Ds => true,
            Tone::Fs => true,
            Tone::Gs => true,
            Tone::As => true,
            _ => false,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Tone::C => "C",
            Tone::Cs => "C#",
            Tone::D => "D",
            Tone::Ds => "D#",
            Tone::E => "E",
            Tone::F => "F",
            Tone::Fs => "F#",
            Tone::G => "G",
            Tone::Gs => "G#",
            Tone::A => "A",
            Tone::As => "A#",
            Tone::B => "B",
        }
    }
}

#[derive(Clone, Copy)]
struct Note {
    octave: Octave,
    tone: Tone,
}

impl Note {
    ///relative positions to A4 (440hz)
    fn offset(&self) -> i32 {
        self.tone as i32 + ((self.octave.index as i32 - 4) * 12) - 9
    }

    fn freq(&self) -> f32 {
        440.0 * 2f32.powf(self.offset() as f32 / 12.0)
    }

    fn to_string(&self) -> String {
        format!("{}{}", self.tone.as_str(), self.octave.index)
    }
}

#[derive(Clone, Copy)]
struct Octave {
    index: u32,
}

impl Octave {
    pub fn notes(&self) -> Vec<Note> {
        Tone::iter()
            .map(|tone| Note {
                octave: *self,
                tone,
            })
            .collect()
    }
}

pub struct KeyboardOutput;

impl Port for KeyboardOutput {
    type Type = f32;

    fn name() -> &'static str {
        "out freq"
    }
}

pub struct Keyboard {
    pressed: Option<Note>,
    key_visuals: Widgets,
    sharp_visuals: Widgets,
}

impl Default for Keyboard {
    fn default() -> Self {
        let mut key_visuals = Widgets::default();
        let mut sharp_visuals = Widgets::default();

        key_visuals.inactive.weak_bg_fill = Color32::WHITE;
        key_visuals.inactive.fg_stroke.color = Color32::DARK_GRAY;
        key_visuals.hovered.fg_stroke.color = Color32::DARK_GRAY;
        key_visuals.hovered.weak_bg_fill = Color32::LIGHT_GRAY;
        key_visuals.active.weak_bg_fill = Color32::GRAY;

        sharp_visuals.inactive.weak_bg_fill = Color32::BLACK;
        sharp_visuals.hovered.weak_bg_fill = Hsva::new(0.0, 0.0, 0.01, 1.0).into();

        Self {
            pressed: None,
            key_visuals,
            sharp_visuals,
        }
    }
}

impl Module for Keyboard {
    fn describe() -> ModuleDescription
    where
        Self: Sized,
    {
        ModuleDescription::new(|| Keyboard::default())
            .set_name("🎹 Keyboard")
            .add_output::<KeyboardOutput>()
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        if let Some(pressed) = self.pressed {
            ctx.set_output::<KeyboardOutput>(pressed.freq())
        } else {
            ctx.set_output::<KeyboardOutput>(0.0)
        }
    }

    fn show(&mut self, _: &ShowContext, ui: &mut Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.set_height(100.0);
            ui.with_layout(
                Layout::left_to_right(eframe::emath::Align::BOTTOM).with_cross_justify(true),
                |ui| {
                    for i in 0..9 {
                        let octave = Octave { index: i };
                        for note in octave.notes() {
                            if note.tone.is_sharp() {
                                ui.style_mut().visuals.widgets = self.sharp_visuals.clone();
                            } else {
                                ui.style_mut().visuals.widgets = self.key_visuals.clone();
                            }

                            ui.style_mut().spacing.item_spacing = Vec2::splat(2.0);

                            let text = if note.tone.is_sharp() {
                                note.tone.as_str().to_string()
                            } else {
                                note.to_string()
                            };

                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new(text).monospace())
                                        .sense(egui::Sense::drag()),
                                )
                                .dragged()
                            {
                                self.pressed = Some(note)
                            }

                            ui.reset_style();
                        }
                    }
                },
            )
        });

        if !ui.memory(|memory| memory.is_anything_being_dragged()) {
            self.pressed = None;
        }
    }
}
