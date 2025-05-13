use forms::{grid::*, *};
use std::rc::Rc;
use tracing::debug;

mod list_view;

const IDC_DEMO_LIST_VIEW: ControlId = ControlId(1);
const IDC_DEMO_TREE_VIEW: ControlId = ControlId(2);
const IDC_DEMO_TAB_CONTROL: ControlId = ControlId(3);
const IDC_DEMO_BUTTONS: ControlId = ControlId(4);

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

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
                let b = Button::new(&form, IDC_DEMO_LIST_VIEW);
                b.set_text("List View");
                GridItem::new(0, 0, LayoutItem::Control(b))
            },
            {
                let b = Button::new(&form, IDC_DEMO_TREE_VIEW);
                b.set_text("Tree View");
                GridItem::new(0, 1, LayoutItem::Control(b))
            },
            {
                let b = Button::new(&form, IDC_DEMO_TAB_CONTROL);
                b.set_text("Tab Control");
                GridItem::new(1, 0, LayoutItem::Control(b))
            },
            {
                let b = Button::new(&form, IDC_DEMO_BUTTONS);
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

    let real_form = form;

    let form = Rc::clone(&real_form);

    real_form.command_handler(Box::new(move |control, command| match (control, command) {
        (IDC_DEMO_TREE_VIEW, Command::ButtonClicked) => {
            debug!("demoing tree view");
        }

        (IDC_DEMO_LIST_VIEW, Command::ButtonClicked) => {
            debug!("demoing list view");
            list_view::demo_list_view(&form);
        }

        _ => {
            debug!("command handler: {control:?} {command:?}");
        }
    }));

    real_form.show_modal();
}
