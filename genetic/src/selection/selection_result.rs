use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SelectionError {
    #[error("Invalid selection: Expecting {0} result but only {1} available")]
    InvalidSelection(usize, usize),
}

pub type SelectionResult = Result<Vec<usize>, SelectionError>;
