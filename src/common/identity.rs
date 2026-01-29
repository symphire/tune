#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Generation(pub(crate) u64);

impl Generation {
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SemanticKey(pub(crate) u64);

impl SemanticKey {
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub struct WithGen<T> {
    pub generation: Generation,
    pub body: T,
}

impl<T> WithGen<T> {
    pub fn new(generation: Generation, body: T) -> WithGen<T> {
        WithGen { generation, body }
    }
}

#[derive(Debug)]
pub struct WithKey<T> {
    pub key: SemanticKey,
    pub body: T,
}

impl<T> WithKey<T> {
    pub fn new(key: SemanticKey, body: T) -> Self {
        WithKey { key, body }
    }
}

#[derive(Debug)]
pub struct WithGenAndKey<T> {
    pub generation: u64,
    pub key: SemanticKey,
    pub body: T,
}

impl<T> WithGenAndKey<T> {
    pub fn new(generation: u64, key: SemanticKey, body: T) -> Self {
        WithGenAndKey {
            generation,
            key,
            body,
        }
    }
}

#[derive(Debug)]
pub struct MaybeWithGen<T> {
    pub generation: Option<u64>,
    pub slot: T,
}

impl<T> MaybeWithGen<T> {
    pub fn new(generation: Option<u64>, slot: T) -> Self {
        Self { generation, slot }
    }
}
