use ulid::Ulid;

#[derive(Clone)]
pub struct Commander {
    id: Ulid,
    name: String,
}