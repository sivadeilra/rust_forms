use forms::custom::CustomControl;
use forms::*;
use std::cell::Cell;

pub fn main() {
    let form = Form::builder()
        .size(1024, 768)
        .text("Custom Control Demo")
        .build();

    let custom = CustomControl::new(
        &form,
        MyStuff {
            mouse_pos: Default::default(),
        },
    );

    form.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().auto(),
        cols: GridAxis::new().auto(),
        items: vec![GridItem::control(0, 0, custom.clone())],
    }));

    form.show_modal();
}

struct MyStuff {
    mouse_pos: Cell<Option<POINT>>,
}

impl custom::CustomInner for MyStuff {
    fn paint(&self, _control: &CustomControl<Self>, dc: &gdi::dc::Dc, _rect: &Rect) {
        dc.text_out_a(20, 20, "what up".as_bytes());

        if let Some(pos) = self.mouse_pos.get() {
            dc.begin_path();
            dc.move_to(pos.x, pos.y - 50);
            dc.line_to(pos.x + 50, pos.y);
            dc.line_to(pos.x, pos.y + 50);
            dc.line_to(pos.x - 50, pos.y);
            dc.close_figure();
            dc.end_path();
            dc.set_pen_color(ColorRef::GREEN);
            dc.stroke_path();

            let msg = format!("{}, {}", pos.x, pos.y);
            dc.text_out_a(pos.x - 20, pos.y - 15, msg.as_bytes());
        }
    }

    fn mouse_move(&self, control: &CustomControl<Self>, pt: POINT) {
        self.mouse_pos.set(Some(pt));
        control.invalidate_all();
    }

    fn mouse_leave(&self, control: &CustomControl<Self>) {
        println!("mouse_leave");
        self.mouse_pos.set(None);
        control.invalidate_all();
    }
}
