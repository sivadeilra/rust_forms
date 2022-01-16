use forms::*;

fn main() {

    env_logger::init();

    init_common_controls();

    let form = Form::builder().text("Hello, world!").size(800, 600).build();

    let edit = TextBox::new(&form, &Rect {
        left: 10,
        right: 110,
        top: 10,
        bottom: 40,


    });

    edit.set_text("Hey, pardner!");

    event_loop();

}
