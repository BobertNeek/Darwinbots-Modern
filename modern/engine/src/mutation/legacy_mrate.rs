//! Original HDRoutines.Save_mrates/Load_mrates: two Integer preferences,
//! followed by eleven rate/mean/deviation Single triples. No enable flags or
//! category distributions are stored in this standalone sidecar.
use super::{MutationRate, StandardMutationKind as Kind, StandardMutationProfile};
use crate::EngineError;

const ORDER: [Kind; 11] = [
    Kind::Point,
    Kind::MinorDeletion,
    Kind::Reversal,
    Kind::Insertion,
    Kind::Amplification,
    Kind::MajorDeletion,
    Kind::CopyError,
    Kind::Delta,
    Kind::Translocation,
    Kind::Point2,
    Kind::CopyError2,
];

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::Invariant(message.into())
}

fn preference(value: &str) -> Result<u8, EngineError> {
    value
        .trim()
        .parse::<i16>()
        .ok()
        .filter(|value| (0..=100).contains(value))
        .map(|value| value as u8)
        .ok_or_else(|| invalid("original mutation preference must be an integer from 0 to 100"))
}

fn single(value: &str, nonnegative: bool) -> Result<f64, EngineError> {
    let value = value
        .trim()
        .parse::<f32>()
        .map_err(|_| invalid("original mutation-rate record is not a Single"))?;
    if !value.is_finite() || (nonnegative && value < 0.0) {
        return Err(invalid(
            "original mutation-rate Single must be finite and valid for its field",
        ));
    }
    Ok(f64::from(value))
}

fn native_single(value: f64, nonnegative: bool) -> Result<f32, EngineError> {
    let native = value as f32;
    if !value.is_finite() || !native.is_finite() || (nonnegative && value < 0.0) {
        return Err(invalid(
            "mutation-rate value is not representable as a valid original Single",
        ));
    }
    Ok(native)
}

impl StandardMutationProfile {
    /// Decode an original standalone .mrate text image with default caller-owned
    /// distributions. This string API performs no filesystem access.
    pub fn from_mrate(source: &str) -> Result<Self, EngineError> {
        Self::from_mrate_with_basis(source, &Self::default())
    }

    /// Preserve distributions that the original sidecar does not contain.
    pub fn from_mrate_with_basis(source: &str, basis: &Self) -> Result<Self, EngineError> {
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
        let records: Vec<_> = normalized
            .strip_suffix('\n')
            .unwrap_or(&normalized)
            .split('\n')
            .collect();
        if records.len() != 35 {
            return Err(invalid(
                "original mutation-rate files require exactly 35 numeric records",
            ));
        }
        let mut profile = basis.clone();
        profile.point_value_change_percent = preference(records[0])?;
        profile.copy_error_value_change_percent = preference(records[1])?;
        for (index, kind) in ORDER.into_iter().enumerate() {
            let start = 2 + 3 * index;
            *profile.rate_mut(kind) = MutationRate {
                one_in: single(records[start], true)?,
                mean: single(records[start + 1], false)?,
                std_dev: single(records[start + 2], true)?,
            };
        }
        profile.validate()?;
        Ok(profile)
    }

    /// Encode exactly the original 35 records, invariant decimal and CRLF.
    /// Numeric values are rounded to the original Single representation first;
    /// distributions and mutation-enabled flags are not part of this format.
    pub fn to_mrate(&self) -> Result<String, EngineError> {
        if self.point_value_change_percent > 100 || self.copy_error_value_change_percent > 100 {
            return Err(invalid(
                "original mutation preference must be an integer from 0 to 100",
            ));
        }
        let mut fields = vec![
            self.point_value_change_percent.to_string(),
            self.copy_error_value_change_percent.to_string(),
        ];
        for kind in ORDER {
            let rate = self.rate(kind);
            fields.push(native_single(rate.one_in, true)?.to_string());
            fields.push(native_single(rate.mean, false)?.to_string());
            fields.push(native_single(rate.std_dev, true)?.to_string());
        }
        Ok(fields.join("\r\n") + "\r\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MutationCategoryDistribution;

    const GOLDEN: &str = "23\r\n67\r\n101\r\n1\r\n0.25\r\n102\r\n2\r\n0.5\r\n103\r\n3\r\n0.75\r\n104\r\n4\r\n1\r\n105\r\n5\r\n1.25\r\n106\r\n6\r\n1.5\r\n107\r\n7\r\n1.75\r\n108\r\n8\r\n2\r\n109\r\n9\r\n2.25\r\n110\r\n10\r\n2.5\r\n111\r\n11\r\n2.75\r\n";

    #[test]
    fn original_mrate_literal_golden_maps_all_eleven_source_slots() {
        let profile = StandardMutationProfile::from_mrate(GOLDEN).unwrap();
        assert_eq!(profile.point_value_change_percent, 23);
        assert_eq!(profile.copy_error_value_change_percent, 67);
        assert_eq!(profile.point, MutationRate::new(101.0, 1.0, 0.25));
        assert_eq!(profile.minor_deletion, MutationRate::new(102.0, 2.0, 0.5));
        assert_eq!(profile.reversal, MutationRate::new(103.0, 3.0, 0.75));
        assert_eq!(profile.insertion, MutationRate::new(104.0, 4.0, 1.0));
        assert_eq!(profile.amplification, MutationRate::new(105.0, 5.0, 1.25));
        assert_eq!(profile.major_deletion, MutationRate::new(106.0, 6.0, 1.5));
        assert_eq!(profile.copy_error, MutationRate::new(107.0, 7.0, 1.75));
        assert_eq!(profile.delta, MutationRate::new(108.0, 8.0, 2.0));
        assert_eq!(profile.translocation, MutationRate::new(109.0, 9.0, 2.25));
        assert_eq!(profile.point2, MutationRate::new(110.0, 10.0, 2.5));
        assert_eq!(profile.copy_error2, MutationRate::new(111.0, 11.0, 2.75));
        assert_eq!(profile.to_mrate().unwrap(), GOLDEN);
        for image in [
            GOLDEN.replace("\r\n", "\n"),
            GOLDEN.replace("\r\n", "\r"),
            GOLDEN.trim_end().to_owned(),
        ] {
            assert_eq!(
                StandardMutationProfile::from_mrate(&image).unwrap(),
                profile
            );
        }
    }

    #[test]
    fn sidecar_preserves_unrepresented_caller_distributions() {
        let basis = StandardMutationProfile {
            point_category_distribution: MutationCategoryDistribution {
                number: 100.0,
                dereference: 0.0,
                operator: 0.0,
                logic: 0.0,
                comparison: 0.0,
                flow: 0.0,
            },
            ..Default::default()
        };
        let profile = StandardMutationProfile::from_mrate_with_basis(GOLDEN, &basis).unwrap();
        assert_eq!(
            profile.point_category_distribution,
            basis.point_category_distribution
        );
        assert_eq!(
            profile.copy_error_category_distribution,
            basis.copy_error_category_distribution
        );
        assert_eq!(basis.point.one_in, 5000.0);
    }

    #[test]
    fn malformed_nonfinite_and_out_of_single_range_records_are_rejected() {
        for image in [
            GOLDEN.to_owned() + "\r\n",
            GOLDEN.to_owned() + "0\r\n",
            GOLDEN.replacen("23\r\n", "23.5\r\n", 1),
            GOLDEN.replacen("23\r\n", "101\r\n", 1),
            GOLDEN.replacen("101\r\n", "\r\n", 1),
        ] {
            assert!(StandardMutationProfile::from_mrate(&image).is_err());
        }
        for token in ["NaN", "Infinity", "-inf", "1e100", "-1", "1,000", "\"101\""] {
            let image = GOLDEN.replacen("101\r\n", &format!("{token}\r\n"), 1);
            assert!(
                StandardMutationProfile::from_mrate(&image).is_err(),
                "{token}"
            );
        }
        let negative_mean = GOLDEN.replacen("1\r\n0.25", "-1\r\n0.25", 1);
        assert_eq!(
            StandardMutationProfile::from_mrate(&negative_mean)
                .unwrap()
                .point
                .mean,
            -1.0
        );
    }

    #[test]
    fn write_number_syntax_accepts_integer_signs_and_single_exponents_invariantly() {
        let image = GOLDEN.replacen(
            "23\r\n67\r\n101\r\n1\r\n0.25\r\n",
            " +23 \r\n-0\r\n +1.01E+02 \r\n-1.0e+0\r\n.25\r\n",
            1,
        );
        let profile = StandardMutationProfile::from_mrate(&image).unwrap();
        assert_eq!(profile.point_value_change_percent, 23);
        assert_eq!(profile.copy_error_value_change_percent, 0);
        assert_eq!(profile.point, MutationRate::new(101.0, -1.0, 0.25));
        for preference in ["23.0", "2.3e1", "23,0", "23 0"] {
            let invalid = GOLDEN.replacen("23\r\n", &format!("{preference}\r\n"), 1);
            assert!(
                StandardMutationProfile::from_mrate(&invalid).is_err(),
                "{preference}"
            );
        }
        assert_eq!(
            StandardMutationProfile::from_mrate(&profile.to_mrate().unwrap()).unwrap(),
            profile
        );
    }

    #[test]
    fn codec_rounds_to_single_and_preserves_single_precision_on_reload() {
        let image = GOLDEN.replacen("101\r\n", "16777217\r\n", 1).replacen(
            "0.25\r\n",
            "0.10000000149011612\r\n",
            1,
        );
        let profile = StandardMutationProfile::from_mrate(&image).unwrap();
        assert_eq!(profile.point.one_in, 16_777_216.0);
        assert_eq!(profile.point.std_dev, f64::from(0.1_f32));
        assert_eq!(
            StandardMutationProfile::from_mrate(&profile.to_mrate().unwrap()).unwrap(),
            profile
        );
        let wide = StandardMutationProfile {
            point: MutationRate::new(16_777_217.0, 0.1, 0.1),
            ..Default::default()
        };
        let projected = StandardMutationProfile::from_mrate(&wide.to_mrate().unwrap()).unwrap();
        assert_eq!(projected.point.one_in, 16_777_216.0);
        assert_eq!(projected.point.mean, f64::from(0.1_f32));
        for one_in in [f64::NAN, f64::INFINITY, f64::MAX, -1.0] {
            assert!(
                StandardMutationProfile {
                    point: MutationRate::new(one_in, 1.0, 0.0),
                    ..Default::default()
                }
                .to_mrate()
                .is_err()
            );
        }
    }
}
