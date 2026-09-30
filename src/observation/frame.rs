use super::schema1;

pub enum Frame {
    Schema1(schema1::Frame),
}

impl Frame {
    pub fn history(&self) -> Self {
        match self {
            Self::Schema1(frame) => Self::Schema1(frame.history()),
        }
    }
}
