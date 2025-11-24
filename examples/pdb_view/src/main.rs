mod modules_view;
mod pdb_ken;
mod symbols_view;
mod text_form;

use std::collections::HashMap;
use std::rc::Rc;

use forms::{App, AppEvent, ControlId, Font, Form, ListView, Notify, With, control_ids};
use ms_pdb::Pdb;
use tracing::{debug, error};

use crate::modules_view::{MODULES_COLUMN_ID, ModulesForm};
use crate::pdb_ken::PdbKen;
use crate::symbols_view::SymbolsForm;
use crate::text_form::TextForm;

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

    text_views: HashMap<String, TextForm>,
    text_views_font: Rc<Font>,

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
            text_views: Default::default(),
            text_views_font: Font::new("Consolas", 16).unwrap(),
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
                self.on_modules_list_double_click();
            }

            AppEvent::Notify { control, notify } => {
                debug!(?control, ?notify, "notify");
            }
        }
    }

    fn on_modules_list_double_click(&mut self) {
        let Some(pdb) = self.pdb.as_ref() else {
            debug!("no pdb active");
            return;
        };

        let Some(selected) = self.modules_form.list_view.iter_selected_items().next() else {
            debug!("no module selected");
            return;
        };

        let id_text = self
            .modules_form
            .list_view
            .get_item_text(selected, MODULES_COLUMN_ID as _);

        let Ok(module_index) = id_text.parse() else {
            // well that is a surprise
            return;
        };

        let _: u32 = module_index;
        debug!(module_index);

        let text_view_name = format!("module/{module_index}");

        if let Some(text_view) = self.text_views.get(&text_view_name) {
            debug!("found existing text view, raising to top");
            text_view.form.raise_to_top();

            self.main_form
                .mdi_client()
                .unwrap()
                .activate_child(&text_view.form);
            return;
        }

        debug!("creating view...");

        let Some(module) = pdb.modules.get(module_index as usize) else {
            debug!("module index out of range");
            return;
        };

        let text_form = TextForm::new(&self.main_form);

        let mut s = String::new();

        use std::fmt::Write;

        _ = writeln!(s, "Module #{module_index}\r\n");
        _ = writeln!(s, "\r\n");
        _ = writeln!(s, "Module name: {}\r\n", module.module_name);
        _ = writeln!(s, "Object file: {}\r\n", module.obj_file);
        _ = writeln!(s);

        if let Some(stream) = module.header.stream() {
            _ = writeln!(s, "Module stream: {stream}\r\n");
        } else {
            _ = writeln!(s, "(no module stream)\r\n");
        };

        text_form.text_edit.set_font(&self.text_views_font);
        text_form.text_edit.set_text(&s);

        self.text_views.insert(text_view_name, text_form);
    }
}
