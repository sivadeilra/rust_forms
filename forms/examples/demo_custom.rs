use forms::custom::CustomControl;
use forms::*;
use log::debug;
use windows::Win32::Foundation::{RECT, COLORREF};
use windows::Win32::Graphics::Gdi::{GetStockObject, CreateSolidBrush};

struct MyStuff {
    x: u32,
}

impl custom::CustomInner for MyStuff {
    fn paint(&self, dc: &gdi::dc::Dc, _rect: &RECT) {

        dc.begin_path();
        dc.move_to(10, 10);
        dc.line_to(20, 10);
        dc.line_to(20, 20);
        dc.end_path();
        dc.set_pen_color(ColorRef::BLUE);
        dc.stroke_path();

        dc.begin_path();
        dc.move_to(50, 50);
        dc.line_to(50, 80);
        dc.line_to(80, 80);
        dc.close_figure();
        dc.end_path();
        dc.set_pen_color(ColorRef::GREEN);
        dc.stroke_path();

        let hbrush =
        unsafe {
            CreateSolidBrush(COLORREF(0xee_ee_ee_ee))
        };
        dc.select_object(hbrush.0);
        dc.fill_path();

        dc.text_out_a(20, 20, "what up".as_bytes());
    }
}

pub fn main() {
    env_logger::builder().format_timestamp(None).init();

    let form = Form::builder()
        .size(1024, 768)
        .text("Custom Control Demo")
        .build();

    let custom = CustomControl::new(&form, MyStuff { x: 42 });

    form.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().auto(),
        cols: GridAxis::new().auto(),
        items: vec![GridItem::control(0, 0, custom.clone())],
    }));

    {
        form.command_handler(move |control, command| match (control, command) {
            _ => {}
        });
    }

    form.notify_handler(move |_notify: &Notify| {
        // ...
    });

    form.show_modal();
}
