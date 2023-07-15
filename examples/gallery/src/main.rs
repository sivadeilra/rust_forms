use forms::{grid::*, *};
use std::rc::Rc;

mod list_view;

fn main() {
    let form = Form::builder()
        .size(1024, 768)
        .quit_on_close()
        .text("Gallery")
        .build();

    form.set_layout(Layout::Grid(GridLayout {
        cols: GridAxis {
            cells: vec![
                GridAxisCell::fixed(100),
                GridAxisCell::fixed(100),
                GridAxisCell::fixed(100),
                GridAxisCell::auto(80),
            ],
            padding: 20,
            lead_margin: 10,
            tail_margin: 10,
        },
        rows: GridAxis {
            cells: vec![
                GridAxisCell::fixed(40),
                GridAxisCell::fixed(40),
                GridAxisCell::auto(30),
            ],
            padding: 20,
            lead_margin: 10,
            tail_margin: 10,
        },
        items: vec![
            {
                let b = Button::new(&form);
                b.set_text("List View");
                let form = Rc::clone(&form);
                b.on_clicked(EventHandler::new(move |_args| {
                    list_view::demo_list_view(&form);
                }));
                GridItem::new(0, 0, LayoutItem::Control(b))
            },
            {
                let b = Button::new(&form);
                b.set_text("Tree View");
                b.on_clicked(EventHandler::new(|_args| {
                    // todo
                }));
                GridItem::new(0, 1, LayoutItem::Control(b))
            },
            {
                let b = Button::new(&form);
                b.set_text("Tab Control");
                GridItem::new(1, 0, LayoutItem::Control(b))
            },
            {
                let b = Button::new(&form);
                b.set_text("Buttons");
                GridItem::new(1, 1, LayoutItem::Control(b))
            },
            {
                let t = TabControl::new(&form);
                t.add_tab(0, "Hello");
                t.add_tab(1, "World");
                GridItem {
                    col_span: 3,
                    ..GridItem::new(2, 0, LayoutItem::Control(t))
                }
            },
        ],
    }));

    form.show_modal();
}
