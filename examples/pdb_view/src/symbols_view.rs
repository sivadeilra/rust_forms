use anyhow::Result;
use ms_pdb::BStr;
use ms_pdb::codeview::IteratorWithRangesExt;
use ms_pdb::syms::{OffsetSegment, SymData, SymIter, SymKind};
use std::rc::Rc;
use zerocopy::IntoBytes;

use super::*;
use forms::*;

/// Shows a list of symbols and allows the user to filter them.
///
/// Has a list view which shows the result of symbols queries.
/// Allows filtering by section, module index, module name, symbol name.
pub struct SymbolsForm {
    #[allow(dead_code)]
    form: Form,
    list_view: Rc<ListView>,
    module_filter_edit: Rc<Edit>,
    symbol_name_filter_edit: Rc<Edit>,
}

// const COLUMN_ID: u32 = 0;
const COLUMN_MODULE: u32 = 1;
const COLUMN_SECTION: u32 = 2;
const COLUMN_OFFSET: u32 = 3;
const COLUMN_KIND: u32 = 4;
const COLUKN_NAME: u32 = 5;

impl SymbolsForm {
    pub fn new(parent: &Form) -> Self {
        let form = parent
            .app()
            .form_builder()
            .with(|b| {
                b.title("Symbols");
                b.mdi_parent(parent);
                b.size(1024, 768);
            })
            .build();

        let search_button = Button::new(&form, IDC_SYMBOLS_SEARCH_BUTTON).with(|b| {
            b.set_text("Search");
        });

        let module_filter_label = Label::new(&form, "Module filter:");
        let module_filter_edit = Edit::new(&form, IDC_SYMBOLS_MODULE_FILTER_EDIT);
        let symbol_name_filter_label = Label::new(&form, "Symbol name filter:");
        let symbol_name_filter_edit = Edit::new(&form, IDC_SYMBOLS_SYMBOL_NAME_FILTER_EDIT);

        let list_view = ListView::new(&form).with(|b| {
            b.set_mode(Mode::Details);
            b.set_tab_stop(true);
            b.set_full_row_select(true);
            b.set_grid_lines(true);
            b.add_column(0, 80, "Id"); // arbitrary index
            b.add_column(1, 80, "Module");
            b.add_column(2, 80, "Section");
            b.add_column(3, 80, "Offset");
            b.add_column(4, 80, "Kind");
            b.add_column(5, 500, "Name");
        });

        // "Search:"  [_____________]   [_Search_]
        // "Module filter:

        form.set_layout(Layout::Grid(GridLayout {
            cols: GridAxis::new().fixed(120).auto().fixed(120),
            rows: GridAxis::new()
                .fixed(30)
                .fixed(30)
                .fixed(30)
                .fixed(30)
                .auto(),
            items: vec![
                GridItem::control(1, 2, &search_button).row_span(2),
                GridItem::control(1, 0, &module_filter_label),
                GridItem::control(1, 1, &module_filter_edit),
                GridItem::control(2, 0, &symbol_name_filter_label),
                GridItem::control(2, 1, &symbol_name_filter_edit),
                GridItem::control(4, 0, &list_view).col_span(3),
            ],
        }));

        Self {
            form,
            list_view,
            module_filter_edit,
            symbol_name_filter_edit,
        }
    }

    pub fn on_search(&mut self, pdb: &mut PdbKen) -> Result<()> {
        let module_filter = self.module_filter_edit.get_text();
        let module_filter = module_filter.trim_ascii();

        let symbol_name_filter = self.symbol_name_filter_edit.get_text();
        let symbol_name_filter = symbol_name_filter.trim_ascii();

        self.list_view.delete_all_items();

        // If there is a module filter, then find that module now.

        let mut module_filter_index: Vec<usize> = Vec::new();

        if !module_filter.is_empty() {
            for (module_index, module) in pdb.modules.iter().enumerate() {
                if module.module_name.contains(module_filter) {
                    debug!(module_index, "module matches by name");
                } else if module.obj_file.contains(module_filter) {
                    debug!(module_index, "module matches by obj_file");
                } else {
                    continue;
                }
                module_filter_index.push(module_index);
            }

            if module_filter_index.is_empty() {
                debug!("there are no modules that match the filter");
                return Ok(());
            }
        }

        // Set up iterators for walking through module list, either filtered or all of them.

        let mut module_filter_iter = if module_filter_index.is_empty() {
            None
        } else {
            Some(module_filter_index.iter())
        };
        let mut next_module_index: usize = 0;

        let num_modules = pdb.modules.len();
        let mut next_module_index = move || -> Option<usize> {
            if let Some(ref mut iter) = module_filter_iter {
                iter.next().copied()
            } else {
                if next_module_index < num_modules {
                    let next = next_module_index;
                    next_module_index += 1;
                    Some(next)
                } else {
                    None
                }
            }
        };

        // turn off list view updates for bulk insert
        self.list_view.set_visible(false);

        // Loop once per module
        while let Some(module_index) = next_module_index() {
            pdb.load_module_symbols(module_index)?;

            let Some(module_symbols) = pdb.get_module_symbols(module_index) else {
                continue;
            };

            if module_symbols.is_empty() {
                continue;
            }

            // The [1..] skips the 4-byte header for module symbols.
            for (sym_range, sym) in SymIter::new(module_symbols[1..].as_bytes()).with_ranges() {
                let Ok(data) = sym.parse() else {
                    continue;
                };

                let offset_seg: OffsetSegment;
                let name: &BStr;
                let kind: &str;

                match (sym.kind, data) {
                    (SymKind::S_GDATA32, SymData::Data(data)) => {
                        offset_seg = data.header.offset_segment;
                        name = data.name;
                        kind = "S_GDATA32";
                    }

                    (SymKind::S_LDATA32, SymData::Data(data)) => {
                        offset_seg = data.header.offset_segment;
                        name = data.name;
                        kind = "S_LDATA32";
                    }

                    (SymKind::S_GPROC32, SymData::Proc(proc)) => {
                        offset_seg = proc.fixed.offset_segment;
                        name = proc.name;
                        kind = "S_GPROC32";
                    }

                    (SymKind::S_LPROC32, SymData::Proc(proc)) => {
                        offset_seg = proc.fixed.offset_segment;
                        name = proc.name;
                        kind = "S_LPROC32";
                    }

                    _ => {
                        // Not supported
                        continue;
                    }
                }

                let Ok(name) = str::from_utf8(name.as_bytes()) else {
                    debug!("symbol name is not valid utf8");
                    continue;
                };

                if !symbol_name_filter.is_empty() {
                    if !name.contains(symbol_name_filter) {
                        continue;
                    }
                    debug!(name = name.to_string(), "symbol name matches");
                }

                // add the symbol to the view
                let id_text = format!("{:08x}", sym_range.start);
                let ii = self.list_view.insert_item(&id_text);
                // self.list_view.set_subitem_text(ii, COLUMN_ID, "");
                self.list_view.set_subitem_text(
                    ii,
                    COLUMN_MODULE as usize,
                    &format!("{module_index}"),
                );
                self.list_view.set_subitem_text(
                    ii,
                    COLUMN_SECTION as usize,
                    &format!("{}", offset_seg.segment.get()),
                );
                self.list_view.set_subitem_text(
                    ii,
                    COLUMN_OFFSET as usize,
                    &format!("{:08x}", offset_seg.offset.get()),
                );
                self.list_view
                    .set_subitem_text(ii, COLUMN_KIND as usize, kind);
                self.list_view
                    .set_subitem_text(ii, COLUKN_NAME as usize, &name.to_string());
            }
        }

        self.list_view.set_visible(true);

        Ok(())
    }
}
