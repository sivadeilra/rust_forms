use std::rc::{Rc, Weak};

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

    let _form_of = form.command_handler(FormData {
        form: Rc::downgrade(&form),
    });

    form.show_modal();
}

struct FormData {
    form: Weak<Form>,
}

impl FormHandler for FormData {
    fn notify(&mut self, control: ControlId, notify: Notify) {
        let form = self.form.upgrade().unwrap();

        match (control, notify) {
            (IDC_DEMO_TREE_VIEW, Notify::ButtonClicked) => {
                debug!("demoing tree view");
            }

            (IDC_DEMO_LIST_VIEW, Notify::ButtonClicked) => {
                debug!("demoing list view");
                list_view::demo_list_view(&form);
            }

            _ => {}
        }
    }
}
