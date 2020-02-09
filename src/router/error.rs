#[derive(Debug, thiserror::Error)]
#[error("{msg}")]
pub struct RouterError {
    msg: &'static str,
}

impl RouterError {
    pub(super) fn new(msg: &'static str) -> Self {
        Self { msg }
    }
}
