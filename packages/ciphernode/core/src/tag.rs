use std::sync::RwLock;

use lazy_static::lazy_static;

lazy_static! {
    static ref TAG: RwLock<String> = RwLock::new(String::from("_"));
}

pub fn get_tag() -> String {
    let tag_guard = TAG.read().expect("Failed to acquire read lock");
    tag_guard.clone()
}

pub fn set_tag(new_tag: impl Into<String>) -> Result<(), &'static str> {
    match TAG.write() {
        Ok(mut tag_guard) => {
            *tag_guard = new_tag.into();
            Ok(())
        }
        Err(_) => Err("Failed to acquire write lock"),
    }
}
