/// Represents the failure of a tests.
///
/// If there was a recording, a shared reference to the temporary location of the recording in
/// sent with the failure report. When the failure is registered to the `Reports`, the later will
/// decide if it needs to keep it by moving the content to the persisting path. When all the
/// failures for a batch have been seen, the Rc is dropped and the temp dir is deleted.
pub struct Failure {
    pub name: String,
    pub message: String,
}
