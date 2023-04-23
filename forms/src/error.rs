use windows::Win32::Foundation::WIN32_ERROR;

#[derive(Debug)]
pub enum Error {
    Windows(WIN32_ERROR),
    ItemDeleted,
    ParentDeleted,
}

pub type Result<T> = core::result::Result<T, Error>;

impl From<WIN32_ERROR> for Error {
    fn from(e: WIN32_ERROR) -> Self {
        Self::Windows(e)
    }
}
