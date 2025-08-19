// This is a placeholder.
pub struct Polynomial<let N: u32> {
    pub coefficients: [Field; N],
}

impl<let N: u32> Polynomial<N> {
    pub fn new(coefficients: [Field; N]) -> Self {
        Polynomial { coefficients }
    }
}
