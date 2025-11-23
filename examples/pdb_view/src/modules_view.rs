use anyhow::Result;
use std::rc::Rc;

use forms::{Button, Edit, GridAxis, GridItem, GridLayout, Layout, Mode, StackLayout};

use super::*;

pub struct ModulesForm {
    #[allow(dead_code)]
    pub form: Form,
    pub list_view: Rc<ListView>,
    pub search_text: Rc<Edit>,
}

pub const MODULES_COLUMN_ID: u32 = 0;
pub const MODULES_COLUMN_OBJECT_FILE: u32 = 1;
pub const MODULES_COLUMN_MODULE_NAME: u32 = 2;

impl ModulesForm {
    pub fn new(parent: &Form) -> Self {
        let form = parent
            .app()
            .form_builder()
            .with(|b| {
                b.mdi_parent(parent);
                b.title("Modules");
                b.size(800, 600);
            })
            .build();

        let list_view = ListView::new(&form, Some(IDC_MODULES_LIST_VIEW)).with(|b| {
            b.set_mode(Mode::Details);
            b.set_grid_lines(true);
            b.set_full_row_select(true);
            b.set_show_selection_always(true);
            b.add_column(MODULES_COLUMN_ID, 30, "Id");
            b.add_column(MODULES_COLUMN_OBJECT_FILE, 200, "Object File");
            b.add_column(MODULES_COLUMN_MODULE_NAME, 400, "Module Name");
        });

        let search_text = Edit::new(&form, IDC_MODULES_SEARCH_EDIT);

        let search_button = Button::new(&form, IDC_MODULES_SEARCH_BUTTON).with(|b| {
            b.set_text("Search");
        });

        let buttons_layout = Layout::Stack(
            StackLayout::vertical(30)
                .control(&search_text)
                .control(&search_button),
        );

        form.set_layout(Layout::Grid(GridLayout {
            rows: GridAxis::new().fixed(50).auto().fixed(50),
            cols: GridAxis::new().auto_min(300).fixed(200),
            items: vec![
                GridItem::control(1, 0, &list_view),
                GridItem::layout(1, 1, buttons_layout),
            ],
        }));

        ModulesForm {
            form,
            list_view,
            search_text,
        }
    }

    pub fn load_pdb(&mut self, pdb: &PdbKen) -> Result<()> {
        let modules = pdb.pdb.modules()?;

        self.list_view.delete_all_items();

        // Hide items because this makes bulk-loading faster.
        self.list_view.set_visible(false);

        for (i, module) in modules.iter().enumerate() {
            let module_index_string = format!("{i}");
            let ii = self.list_view.insert_item(&module_index_string);
            self.list_view
                .set_subitem_text(ii, 1, &module.obj_file().to_string());
            self.list_view
                .set_subitem_text(ii, 2, &module.module_name().to_string());
        }

        self.list_view.set_visible(true);

        Ok(())
    }

    pub fn on_search(&mut self) {
        let search_string = self.search_text.get_text();
        debug!(?search_string, "on_search");

        let search_str = search_string.trim_ascii();
        if search_str.is_empty() {
            return;
        }

        let start_index = if let Some(selected_index) = self.list_view.iter_selected_items().next()
        {
            selected_index + 1
        } else {
            0
        };

        let num_items = self.list_view.items_len();

        let mut ii = start_index;
        let found: usize = loop {
            let obj_file = self.list_view.get_item_text(ii, 1);
            if obj_file.contains(search_str) {
                debug!(found = ii, "found match in obj_file: {obj_file:?}");
                break ii;
            }

            let module_name = self.list_view.get_item_text(ii, 2);
            if module_name.contains(search_str) {
                debug!(found = ii, "found match in module_name: {module_name:?}");
                break ii;
            }

            ii += 1;
            if ii == num_items {
                ii = 0;
            }
            if ii == start_index {
                debug!("no match");
                return;
            }
        };

        debug!("found at index: {found}");
        self.list_view.set_all_selected(false);
        self.list_view.set_item_selected(found, true);
        self.list_view.ensure_visible(found);
    }
}
