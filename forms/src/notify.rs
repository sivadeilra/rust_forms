use super::*;

pub enum Notify<'a> {
    ListViewItemActivate(&'a NMITEMACTIVATE),
    ListViewColumnClick(i32),
    Unknown,
}

impl<'a> Notify<'a> {
    pub(crate) unsafe fn from_nmhdr(nmhdr: *const NMHDR) -> Notify<'a> {
        let hdr = &*nmhdr;

        match hdr.code {
            LVN_ITEMACTIVATE => {
                Self::ListViewItemActivate(&*(nmhdr as *const _ as *const NMITEMACTIVATE))
            }

            LVN_COLUMNCLICK => {
                let nmlistview: &NMLISTVIEW = &*(nmhdr as *const NMLISTVIEW);
                Self::ListViewColumnClick(nmlistview.iSubItem)
            }

            _ => Notify::Unknown,
        }
    }
}
