use rack::{
    app::App,
    io::PortHandle,
    module::Port,
    modules::{
        audio::{Audio, FrameInput},
        oscillator::{FrameOutput, Oscillator},
        scope::{Scope, ScopeInput},
    },
};

fn main() {
    let mut app = App::default();

    let oscil = app.rack.add_module_typed::<Oscillator>();
    let audio = app.rack.add_module_typed::<Audio>();
    let scope = app.rack.add_module_typed::<Scope>();

    // app.rack.get_module_mut(&b).unwrap().volume = 0.0;
    // app.rack.get_module_mut(&c).unwrap().volume = 0.0;

    app.rack
        .connect(
            PortHandle::new(FrameOutput::id(), oscil.as_untyped()),
            PortHandle::new(FrameInput::id(), audio.as_untyped()),
        )
        .unwrap();

    app.rack
        .connect(
            PortHandle::new(FrameOutput::id(), oscil.as_untyped()),
            PortHandle::new(ScopeInput::id(), scope.as_untyped()),
        )
        .unwrap();

    app.run()
}
