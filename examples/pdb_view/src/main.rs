mod modules_view;
mod pdb_ken;
mod symbols_view;
mod text_form;

use forms::{App, AppEvent, ControlId, Form, ListView, Notify, With, control_ids};
use ms_pdb::Pdb;
use tracing::{debug, error};

use crate::modules_view::{MODULES_COLUMN_ID, ModulesForm};
use crate::pdb_ken::PdbKen;
use crate::symbols_view::SymbolsForm;

control_ids! {
    // ModulesForm
    IDC_MODULES_SEARCH_BUTTON,
    IDC_MODULES_SEARCH_EDIT,
    IDC_MODULES_LIST_VIEW,

    // SymbolsForm
    IDC_SYMBOLS_SEARCH_BUTTON,
    IDC_SYMBOLS_MODULE_FILTER_EDIT,
    IDC_SYMBOLS_LIST_VIEW,
    IDC_SYMBOLS_SYMBOL_NAME_FILTER_EDIT,



    IDC_TEXT_FORM_EDIT,
}

fn main() {
    let mut app = PdbView::new();

    _ = app.open_file(r"d:\temp\hello_world.pdb");

    while let Some(event) = app.app.wait_event() {
        app.on_event(event);
    }
}

struct PdbView {
    app: App,

    #[allow(dead_code)]
    main_form: Form,

    modules_form: ModulesForm,

    symbols_form: SymbolsForm,

    pdb: Option<PdbKen>,
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

        let symbols_form = SymbolsForm::new(&main_form);

        Self {
            app,
            main_form,
            modules_form,
            symbols_form,
            pdb: None,
        }
    }

    fn open_file(&mut self, file_name: &str) -> anyhow::Result<()> {
        self.pdb = None;

        let pdb = match Pdb::open(file_name.as_ref()) {
            Ok(pdb) => PdbKen::new(pdb)?,
            Err(e) => {
                error!("failed to open PDB file: {e:?}");
                return Ok(());
            }
        };

        debug!("successfully opened PDB file.");

        _ = self.modules_form.load_pdb(&pdb);

        self.pdb = Some(pdb);
        Ok(())
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

            AppEvent::Notify {
                control: IDC_SYMBOLS_SEARCH_BUTTON,
                notify: Notify::ButtonClicked,
            } => {
                if let Some(pdb) = &mut self.pdb {
                    _ = self.symbols_form.on_search(pdb);
                }
            }

            AppEvent::Notify {
                control: IDC_MODULES_LIST_VIEW,
                notify: Notify::ItemDoubleClick { item, subitem },
            } => {
                debug!(item, subitem, "double-click");

                if let Some(selected) = self.modules_form.list_view.iter_selected_items().next() {
                    let id_text = self
                        .modules_form
                        .list_view
                        .get_item_text(selected, MODULES_COLUMN_ID as _);
                    if let Ok(module_index) = id_text.parse() {
                        let _: u32 = module_index;
                        debug!(module_index);
                    } else {
                        // well that is a surprise
                    }
                } else {
                    // double-clicked on nothing
                }
            }

            AppEvent::Notify { control, notify } => {
                debug!(?control, ?notify, "notify");
            }
        }
    }
}
