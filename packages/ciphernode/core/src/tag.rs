use std::sync::OnceLock;

static TAG: OnceLock<String> = OnceLock::new();

pub fn get_tag() -> String {
    TAG.get().cloned().unwrap_or_else(|| String::from("_"))
}

pub fn set_tag(new_tag: impl Into<String>) -> Result<(), &'static str> {
    TAG.set(new_tag.into())
        .map_err(|_| "Tag has already been initialized")
}
