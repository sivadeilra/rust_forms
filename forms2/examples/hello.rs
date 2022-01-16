use forms2::*;

fn main() {
    env_logger::init();

    let app = App::new();

    let f = Form::builder()
        .size(800, 600)
        .quit_on_close()
        .text("Hello, World")
        .build();

    let lv = ListView::new(&f);

    lv.set_view(Mode::Details);
    lv.add_column(0, "Stuff");
    lv.add_column(1, "More stuff");

    let start = Button::new(&f);
    start.set_text("Start Your Engine");
    start.on_clicked(EventHandler::new(move |()| {
        println!("clicked!");
    }));

    // f.set_title("Hello, world");
    f.show_window();

    app.run();
}
