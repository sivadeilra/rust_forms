use super::*;

pub struct PropertySheet {
}

pub struct PropertyPage {
}

pub struct PropertySheetBuilder {
    pages: Vec<PropertyPageBuilder>,
}

pub struct PropertyPageBuilder {
}

impl PropertySheetBuilder {
    pub fn page(mut self, page: PropertyPageBuilder) -> Self {
        self.pages.push(page);
    }
}

impl PropertySheet {
    pub fn builder() -> PropertySheetBuilder {
        PropertySheetBuilder {
            pages: Vec::with_capacity(10),
        }
    }

}
