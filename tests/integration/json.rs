use biparse::prelude::*;

enum Json {
    Null,
    Boolean(bool),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}
