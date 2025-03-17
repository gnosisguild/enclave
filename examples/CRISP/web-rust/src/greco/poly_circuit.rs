use axiom_eth::rlc::chip::RlcChip;
use axiom_eth::halo2_base::{
    gates::{GateInstructions, RangeChip, RangeInstructions},
    utils::ScalarField,
    AssignedValue, Context,
    QuantumCell::Constant,
};
/// Struct to store the coefficients of a polynomial as Field Elements
/// The coefficients are stored starting from the highest degree term
#[derive(Clone, Debug)]
pub struct Poly<F: ScalarField> {
    pub coefficients: Vec<F>,
}

impl<F: ScalarField> Poly<F> {
    pub fn new(coefficients: Vec<String>) -> Self {
        let coefficients = coefficients
            .iter()
            .map(|coeff| F::from_str_vartime(coeff).unwrap())
            .collect();
        Poly { coefficients }
    }

    /// Evaluate a polynomial at a point in the field.
    pub fn eval(&self, x: F) -> F {
        {
            let mut acc = self.coefficients[0];
            for coeff in self.coefficients.iter().skip(1) {
                acc = acc * x + coeff;
            }
            acc
        }
    }
}

pub struct PolyAssigned<F: ScalarField> {
    pub assigned_coefficients: Vec<AssignedValue<F>>,
}

impl<F: ScalarField> PolyAssigned<F> {
    pub fn new(ctx: &mut Context<F>, poly: Poly<F>) -> Self {
        let assigned_coefficients = ctx.assign_witnesses(poly.coefficients);
        PolyAssigned {
            assigned_coefficients,
        }
    }

    /// Adds `upper_bound` to the coefficients of the polynomial and constrains them to be in the range `[0, 2*upper_bound]`.
    pub fn range_check_1bound(&self, ctx_gate: &mut Context<F>, range: &RangeChip<F>, upper_bound: u64) {
        let bound_constant = Constant(F::from(upper_bound));
        for coeff in &self.assigned_coefficients {
            let shifted_coeff = range.gate().add(ctx_gate, *coeff, bound_constant);
            range.check_less_than_safe(ctx_gate, shifted_coeff, (2 * upper_bound) + 1);

        }
    }


    /// Adds `-lower_bound` to the coefficients of the polynomial and constrains them to be in the range `[0, upper_bound - lower_bound]`.
    pub fn range_check_2bounds(&self, ctx_gate: &mut Context<F>, range: &RangeChip<F>, lower_bound: i64, upper_bound: u64) {
        let value_shift = -&lower_bound as u64;
        let bound_constant = Constant(F::from(value_shift));

        for coeff in &self.assigned_coefficients {
            let shifted_coeff = range.gate().add(ctx_gate, *coeff, bound_constant);
            range.check_less_than_safe(ctx_gate, shifted_coeff, (value_shift + upper_bound) + 1);
        }
    }

    pub fn enforce_eval_at_gamma(
        &self,
        ctx_rlc: &mut Context<F>,
        rlc: &RlcChip<F>,
    ) -> AssignedValue<F> {
        let rlc_trace = rlc.compute_rlc_fixed_len(ctx_rlc, self.assigned_coefficients.clone());
        rlc_trace.rlc_val
    }
}
