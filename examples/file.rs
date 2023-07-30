use rack::{
    app::App,
    io::PortHandle,
    module::Port,
    modules::{
        audio::{Audio, AudioInput},
        file::{File, FileOutput},
        scope::{Scope, ScopeInput},
    },
};

fn main() {
    let mut app = App::default();

    let audio = app.rack.add_module_typed::<Audio>();
    let file = app.rack.add_module_typed::<File>();
    let scope = app.rack.add_module_typed::<Scope>();

    app.rack
        .connect(
            PortHandle::new(FileOutput::id(), file.as_untyped()),
            PortHandle::new(AudioInput::id(), audio.as_untyped()),
        )
        .unwrap();

    app.rack
        .connect(
            PortHandle::new(FileOutput::id(), file.as_untyped()),
            PortHandle::new(ScopeInput::id(), scope.as_untyped()),
        )
        .unwrap();

    app.rack
        .get_module_mut(file)
        .unwrap()
        .open_file("sample.mp3");

    app.run()
}
