use super::*;
use std::borrow::Cow;
use widestring::{U16Str, U16String};
use windows::Win32::UI::Controls::Dialogs::*;

pub struct FileDialog {
    pub allow_multi_select: bool,
    /// A list of extensions, separated by `;`.
    pub filters: FileFilter,
    pub filter_all_files: bool,
    pub dir: Option<String>,
    pub title: Option<String>,
    pub initial_dir: Option<String>,
}

pub enum FileFilter {
    None,
    List(Vec<(Cow<'static, String>, Cow<'static, String>)>),
    Static(&'static [(&'static str, &'static str)]),
}

pub enum FileDialogResult {
    Multi { dir: String, files: Vec<String> },
    Single(String),
    Canceled,
}

impl Default for FileDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDialog {
    pub fn new() -> Self {
        Self {
            allow_multi_select: false,
            filters: FileFilter::None,
            filter_all_files: true,
            dir: None,
            title: None,
            initial_dir: None,
        }
    }

    pub fn open(&self, parent: Option<&Form>) -> FileDialogResult {
        self.show(parent, true)
    }

    pub fn save(&self, parent: Option<&Form>) -> FileDialogResult {
        self.show(parent, false)
    }

    fn show(&self, parent: Option<&Form>, is_open: bool) -> FileDialogResult {
        // https://docs.microsoft.com/en-us/windows/win32/api/commdlg/ns-commdlg-openfilenamew
        unsafe {
            let mut of: OPENFILENAMEW = zeroed();
            of.lStructSize = size_of_val(&of) as u32;
            of.hwndOwner = parent.map(|f| f.handle()).unwrap_or(HWND(0));

            // Filename buffer
            let mut filename_buffer: Vec<u16> = vec![0; 32768];
            of.lpstrFile = PWSTR(filename_buffer.as_mut_ptr());
            of.nMaxFile = filename_buffer.len() as u32;

            // Handle filters
            let mut filters = U16String::new();
            let filters_ptr: *const u16;
            match &self.filters {
                FileFilter::None => {
                    filters_ptr = null();
                }
                FileFilter::List(list) => {
                    for (display, exts) in list.iter() {
                        filters.push_str(&**display);
                        filters.push_str("\0");
                        filters.push_str(&**exts);
                        filters.push_str("\0");
                    }
                    filters.push_str("All files\0*\0");
                    filters.push_str("\0");
                    filters_ptr = filters.as_ptr();
                }
                FileFilter::Static(list) => {
                    for (display, exts) in list.iter() {
                        filters.push_str(display);
                        filters.push_str("\0");
                        filters.push_str(exts);
                        filters.push_str("\0");
                    }
                    filters.push_str("All files\0*\0");
                    filters.push_str("\0");
                    filters_ptr = filters.as_ptr();
                }
            }
            of.lpstrFilter = PCWSTR::from_raw(filters_ptr);
            debug!("filter: {:?}", filters.as_slice());

            // Handle title
            let mut title: U16CString;
            if let Some(t) = self.title.as_ref() {
                title = U16CString::from_str(t).unwrap();
                of.lpstrFileTitle = PWSTR::from_raw(title.as_mut_ptr());
            }

            // Handle initial dir
            let initial_dir: U16CString;
            if let Some(s) = self.initial_dir.as_ref() {
                initial_dir = U16CString::from_str(s).unwrap();
                of.lpstrInitialDir = PCWSTR::from_raw(initial_dir.as_ptr());
            }

            if self.allow_multi_select {
                of.Flags |= OFN_ALLOWMULTISELECT;
            }

            of.Flags |= OFN_EXPLORER;

            let succeeded = if is_open {
                GetOpenFileNameW(&mut of).as_bool()
            } else {
                GetSaveFileNameW(&mut of).as_bool()
            };

            if !succeeded {
                debug!("failed");
                return FileDialogResult::Canceled;
            }

            if self.allow_multi_select {
                let mut filenames: Vec<String> = Vec::new();

                let mut dir_len = of.nFileOffset as usize;
                while dir_len > 0 && filename_buffer[dir_len - 1] == 0 {
                    dir_len -= 1;
                }

                let parent_dir = U16Str::from_slice(&filename_buffer[..dir_len]).to_string_lossy();

                let mut pos = of.nFileOffset as usize;
                loop {
                    if pos >= filename_buffer.len() {
                        break;
                    }
                    if filename_buffer[pos] == 0 {
                        break;
                    }
                    let start = pos;
                    pos += 1;
                    let end;
                    loop {
                        if pos >= filename_buffer.len() {
                            end = pos;
                            break;
                        }
                        if filename_buffer[pos] == 0 {
                            end = pos;
                            pos += 1;
                            break;
                        }
                        pos += 1;
                    }

                    let filename =
                        U16Str::from_slice(&filename_buffer[start..end]).to_string_lossy();
                    filenames.push(filename);
                }
                FileDialogResult::Multi {
                    dir: parent_dir,
                    files: filenames,
                }
            } else {
                let single_filename = widestring::U16CStr::from_slice_truncate(&filename_buffer)
                    .unwrap()
                    .to_string_lossy();
                debug!("filename = {:?}", single_filename);

                FileDialogResult::Single(single_filename)
            }
        }
    }
}
