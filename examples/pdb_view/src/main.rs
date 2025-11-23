//use ms_pdb::Pdb;

mod modules_view;

use forms::{App, AppEvent, ControlId, Form, ListView, Notify, With};
use ms_pdb::Pdb;
use tracing::{debug, error};

use crate::modules_view::ModulesForm;

const IDC_MODULES_SEARCH_BUTTON: ControlId = ControlId(1);
const IDC_MODULES_SEARCH_EDIT: ControlId = ControlId(2);

fn main() {
    let mut app = PdbView::new();

    app.open_file(r"d:\temp\hello_world.pdb");

    while let Some(event) = app.app.wait_event() {
        app.on_event(event);
    }
}

struct PdbView {
    app: App,

    main_form: Form,

    modules_form: ModulesForm,

    pdb: Option<Box<Pdb>>,
}

impl PdbView {
    fn new() -> Self {
        let app = forms::App::new();

        let main_form = app
            .form_builder()
            .with(|b| {
                b.mdi_frame();
                b.size(1280, 1024);
                b.title("PDB Viewer");
            })
            .build();

        let modules_form = ModulesForm::new(&main_form);

        Self {
            app,
            main_form,
            modules_form,
            pdb: None,
        }
    }

    fn open_file(&mut self, file_name: &str) {
        self.pdb = None;

        let pdb = match Pdb::open(file_name.as_ref()) {
            Ok(pdb) => pdb,
            Err(e) => {
                error!("failed to open PDB file: {e:?}");
                return;
            }
        };

        debug!("successfully opened PDB file.");

        self.modules_form.load_pdb(&pdb);

        self.pdb = Some(pdb);
    }

    fn on_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Quit => {}

            AppEvent::Notify {
                control: IDC_MODULES_SEARCH_BUTTON,
                notify: Notify::ButtonClicked,
            } => {
                self.modules_form.on_search();
            }

            AppEvent::Notify { control, notify } => {
                debug!(?control, ?notify, "notify");
            }
        }
    }
}
