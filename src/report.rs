pub enum Report {
    Failures(Vec<Failure>),
    Ok,
}

pub struct Failure {
    pub name: String,
    pub message: String,
}
