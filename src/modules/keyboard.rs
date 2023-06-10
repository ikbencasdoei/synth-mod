use std::fmt::Display;

use eframe::{
    egui::{self, style::Widgets, Layout, Ui},
    epaint::{Color32, Hsva, Vec2},
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    module::{Module, ModuleDescription, Port, PortDescription},
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
        matches!(self, Tone::Cs | Tone::Ds | Tone::Fs | Tone::Gs | Tone::As)
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
    ///Relative position of this `Note` to A4 (440Hz)
    fn offset(&self) -> i32 {
        self.tone as i32 + ((self.octave.index as i32 - 4) * 12) - 9
    }

    fn freq(&self) -> f32 {
        440.0 * 2f32.powf(self.offset() as f32 / 12.0)
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.tone.as_str(), self.octave.index)
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

pub struct KeyboardFreqOutput;

impl Port for KeyboardFreqOutput {
    type Type = f32;

    fn name() -> &'static str {
        "out freq"
    }
}

pub struct KeyboardPressedOutput;

impl Port for KeyboardPressedOutput {
    type Type = bool;

    fn name() -> &'static str {
        "pressed"
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
    fn describe() -> ModuleDescription<Self>
    where
        Self: Sized,
    {
        ModuleDescription::new(Keyboard::default)
            .name("🎹 Keyboard")
            .port(PortDescription::<KeyboardFreqOutput>::output())
            .port(PortDescription::<KeyboardPressedOutput>::output())
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        if let Some(pressed) = self.pressed {
            ctx.set_output::<KeyboardFreqOutput>(pressed.freq());
            ctx.set_output::<KeyboardPressedOutput>(true)
        } else {
            ctx.set_output::<KeyboardFreqOutput>(0.0);
            ctx.set_output::<KeyboardPressedOutput>(false)
        }
    }

    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {
        egui::ScrollArea::horizontal()
            .id_source(ctx.instance)
            .drag_to_scroll(false)
            .show(ui, |ui| {
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
                                    format!("{}", note)
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
