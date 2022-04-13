use serde::{Deserialize, Serialize};

use crate::edit_field::EditField;

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct ArrayWidget {
    entries: Vec<EditField>,
}

impl ArrayWidget {
    pub fn new(entries: Vec<EditField>) -> Self {
        ArrayWidget { entries }
    }
}

impl From<Vec<EditField>> for ArrayWidget {
    fn from(entries: Vec<EditField>) -> Self {
        Self::new(entries)
    }
}
