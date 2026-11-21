pub(crate) fn store_scaled_sum(store: f32, amount: f64, share: f64) -> f32 {
    let store = f64::from(store);
    let credit = amount * share;
    let total = store + credit;
    let rounded = total as f32;
    if total == f64::from(rounded) {
        return rounded;
    }
    let rounds_down = total > f64::from(rounded);
    let other = if rounds_down {
        rounded.next_up()
    } else {
        rounded.next_down()
    };
    if total != (f64::from(rounded) + f64::from(other)) * 0.5 {
        return rounded;
    }

    // Native x87 arithmetic retains a 64-bit significand before the Single
    // assignment. Recover the bits lost by Double at the Single midpoint.
    let credit_step = extended_ulp(credit);
    let credit_remainder =
        (amount.mul_add(share, -credit) / credit_step).round_ties_even() * credit_step;
    let stored_credit = total - store;
    let sum_remainder = (store - (total - stored_credit)) + (credit - stored_credit);
    let remainder = sum_remainder + credit_remainder;
    let half_step = extended_ulp(total) * 0.5;
    if (rounds_down && remainder > half_step) || (!rounds_down && remainder < -half_step) {
        other
    } else {
        rounded
    }
}

fn extended_ulp(value: f64) -> f64 {
    const EXPONENT_MASK: u64 = 0x7ff0_0000_0000_0000;
    let exponent = value.to_bits() & EXPONENT_MASK;
    f64::from_bits(exponent - (63_u64 << 52))
}
