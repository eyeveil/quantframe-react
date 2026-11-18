#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    Unset,
    Value(i64),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BulkEdit<T> {
    #[default]
    Keep,
    Set(T),
}
