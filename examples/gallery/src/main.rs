use std::rc::Rc;

use forms::{grid::*, *};
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

    let app = forms::App::new();

    let form = app
        .form_builder()
        .with(|b| {
            b.size(1024, 768);
            b.quit_on_close();
            b.title("Gallery");
        })
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
                GridItem::control(0, 0, &b)
            },
            {
                let b = Button::new(&form, IDC_DEMO_TREE_VIEW);
                b.set_text("Tree View");
                GridItem::control(0, 1, &b)
            },
            {
                let b = Button::new(&form, IDC_DEMO_TAB_CONTROL);
                b.set_text("Tab Control");
                GridItem::control(1, 0, &b)
            },
            {
                let b = Button::new(&form, IDC_DEMO_BUTTONS);
                b.set_text("Buttons");
                GridItem::control(1, 1, &b)
            },
            {
                let t = TabControl::new(&form);
                t.add_tab(0, "Hello");
                t.add_tab(1, "World");
                GridItem::control(2, 0, &t).col_span(3)
            },
        ],
    }));

    let form = Rc::new(form);

    form.command_handler({
        let form = form.clone();
        Box::new(move |control, command| match (control, command) {
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
        })
    });

    form.show_modal();
}
