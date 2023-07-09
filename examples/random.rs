use rack::app::App;
use rand::Rng;

/// Generate a rack filled with random modules.
fn main() {
    let mut app = App::default();

    let mut handles = Vec::new();
    for panel in 0..5 {
        app.rack.add_panel();
        for _ in 0..10 {
            let choice = rand::thread_rng().gen_range(0..app.rack.modules.len());
            let module = app.rack.modules.get(choice).unwrap().clone();
            handles.push(app.rack.add_module(&module, panel));
        }
    }

    let inputs = handles
        .iter()
        .flat_map(|&handle| app.rack.get_instance(handle))
        .flat_map(|instance| instance.inputs.keys().cloned().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let outputs = handles
        .iter()
        .flat_map(|&handle| app.rack.get_instance(handle))
        .flat_map(|instance| instance.outputs.keys().cloned().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    for input in inputs {
        let choice = rand::thread_rng().gen_range(0..outputs.len());
        let &from = outputs.get(choice).unwrap();
        app.rack.connect(from, input).ok();
    }

    app.run()
}
