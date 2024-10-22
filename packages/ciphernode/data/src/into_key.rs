/// This trait allows our keys to be responsive to multiple inputs
pub trait IntoKey {
    fn into_key(self) -> Vec<u8>;
}

/// Keys can be vectors of String
impl IntoKey for Vec<u8> {
    fn into_key(self) -> Vec<u8> {
        self
    }
}

/// Keys can be vectors of String
impl IntoKey for &Vec<u8> {
    fn into_key(self) -> Vec<u8> {
        self.clone()
    }
}

/// Keys can be vectors of String
impl IntoKey for Vec<String> {
    fn into_key(self) -> Vec<u8> {
        self.join("/").into_bytes()
    }
}

/// Keys can be vectors of &str
impl<'a> IntoKey for Vec<&'a str> {
    fn into_key(self) -> Vec<u8> {
        self.join("/").into_bytes()
    }
}

/// Keys can be String
impl IntoKey for String {
    fn into_key(self) -> Vec<u8> {
        self.into_bytes()
    }
}

/// Keys can be &String
impl IntoKey for &String {
    fn into_key(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

/// Keys can be &str
impl<'a> IntoKey for &'a str {
    fn into_key(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}
