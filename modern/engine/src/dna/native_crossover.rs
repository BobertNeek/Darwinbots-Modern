//! Robots.simplematch, scanfromn and crossover (including eager VB6 IIf draws).
use super::*;

pub(super) fn match_labels(left: &[DnaBlock], right: &[DnaBlock]) -> (Vec<u32>, Vec<u32>) {
    let encoded = |blocks: &[DnaBlock]| std::iter::once(-16646)
        .chain(blocks.iter().copied().map(legacy_nucleotide)).collect::<Vec<_>>();
    let left = encoded(left); let right = encoded(right);
    let mut left_labels = vec![0; left.len()]; let mut right_labels = vec![0; right.len()];
    let (left_end, right_end) = (left.len() as isize - 1, right.len() as isize - 1);
    let (mut i, mut j) = (0_isize, 0_isize);
    let (mut left_start, mut right_start) = (0_isize, 0_isize);
    let (mut left_list, mut right_list) = (Vec::new(), Vec::new());
    let (mut run, mut continuing, mut work) = (0_u32, false, 0_u64);
    loop {
        i = i.min(left_end); j = j.min(right_end);
        left_list.push(left[i as usize]); right_list.push(right[j as usize]);
        let mut found = None;
        for index in 0..left_list.len() {
            if right[j as usize] == left_list[index] { found = Some((true, index)); break; }
            if left[i as usize] == right_list[index] { found = Some((false, index)); break; }
            work += 1;
        }
        if let Some((match_right, index)) = found {
            if match_right { i = left_start + index as isize; }
            else { j = right_start + index as isize; }
            while i <= left_end && j <= right_end {
                if left[i as usize] != right[j as usize] {
                    continuing = false; left_start = i; right_start = j;
                    i -= 1; j -= 1; break;
                }
                if !continuing { run += 1; }
                continuing = true;
                left_labels[i as usize] = run; right_labels[j as usize] = run;
                i += 1; j += 1; work += 1;
            }
            left_list.clear(); right_list.clear();
        }
        i += 1; j += 1; work += 1;
        if (i > left_end && j > right_end) || work > 16000_u64.pow(2) { break; }
    }
    (left_labels, right_labels)
}

fn next_layer(labels: &[u32], start: usize, layer: u32) -> (usize, u32) {
    labels.iter().enumerate().skip(start).find(|(_, value)| **value != layer)
        .map_or((labels.len(), layer), |(index, value)| (index, *value))
}

impl LegacyDna {
    pub(crate) fn crossover_native(
        &self, other: &Self, random: &mut crate::native_random::NativeRandom,
    ) -> Self {
        let (left_labels, right_labels) = match_labels(&self.blocks, &other.blocks);
        let dummy = DnaBlock { class: TokenClass::Number, value: 0 };
        let left: Vec<_> = std::iter::once(dummy).chain(self.blocks.iter().copied()).collect();
        let right: Vec<_> = std::iter::once(dummy).chain(other.blocks.iter().copied()).collect();
        let (mut left_start, mut right_start) = (0, 0);
        let mut output = Vec::new();
        loop {
            let (left_match, _) = next_layer(&left_labels, left_start, 0);
            let (right_match, layer) = next_layer(&right_labels, right_start, 0);
            let left_gap = left_match.saturating_sub(left_start);
            let right_gap = right_match.saturating_sub(right_start);
            match (left_gap > 0, right_gap > 0) {
                (true, true) => {
                    if random.unit() * 2.0 < 1.0 { output.extend_from_slice(&left[left_start..left_match]); }
                    else { output.extend_from_slice(&right[right_start..right_match]); }
                }
                (true, false) => if random.unit() * 2.0 < 1.0 { output.extend_from_slice(&left[left_start..left_match]); },
                (false, true) => if random.unit() * 2.0 < 1.0 { output.extend_from_slice(&right[right_start..right_match]); },
                _ => {},
            }
            if layer == 0 || left_match >= left.len() || right_match >= right.len() { break; }
            let (left_stop, _) = next_layer(&left_labels, left_match, layer);
            let count = (left_stop - left_match).min(right.len() - right_match);
            let use_left = random.unit() * 2.0 < 1.0;
            for offset in 0..count {
                let first = left[left_match + offset]; let second = right[right_match + offset];
                // IIf evaluates its random argument even for small values and
                // operators, although those values use the whole-run choice.
                let value_choice = random.unit() * 2.0 < 1.0;
                let independent_value = first.class == second.class
                    && first.value.saturating_abs() > 999 && second.value.saturating_abs() > 999;
                let use_left_value = if independent_value { value_choice } else { use_left };
                output.push(DnaBlock {
                    class: if use_left { first.class } else { second.class },
                    value: if use_left_value { first.value } else { second.value },
                });
            }
            left_start = left_match + count; right_start = right_match + count;
            if count == 0 { break; }
        }
        // Native arrays retain their dummy element. Rust stores only actual DNA.
        let mut blocks: Vec<_> = output.into_iter().skip(1).take(MAX_DNA_BLOCKS - 1).collect();
        normalize_master_end(&mut blocks);
        let mut child = self.with_blocks(blocks);
        child.ordered_user_variables.extend(other.ordered_user_variables.iter()
            .filter(|(name, _)| !self.user_variables.contains_key(&name.to_ascii_lowercase())).cloned());
        for (name, address) in &other.user_variables { child.user_variables.entry(name.clone()).or_insert(*address); }
        child.new_move |= other.new_move;
        child.compatibility_warnings.extend(other.compatibility_warnings.iter().cloned());
        child.compatibility_warnings.sort(); child.compatibility_warnings.dedup();
        child
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_definitions_keep_source_order_case_and_duplicate_slots_through_save_and_source() {
        let dna = LegacyDna::parse("def Zulu 901\ndef Alpha 902\ndef Zulu 903\nstart .Zulu stop").unwrap();
        let expected = vec![("Zulu".to_owned(), 901), ("Alpha".to_owned(), 902), ("Zulu".to_owned(), 903)];
        assert_eq!(dna.native_user_variables(), expected.as_slice());
        assert_eq!(dna.user_variable("Zulu"), Some(903));
        let saved: LegacyDna = serde_json::from_slice(&serde_json::to_vec(&dna).unwrap()).unwrap();
        assert_eq!(saved.native_user_variables(), expected.as_slice());
        let text = LegacyDna::parse(&dna.to_source()).unwrap();
        assert_eq!(text.native_user_variables(), expected.as_slice());
        assert_eq!(text.blocks(), dna.blocks());
    }

    #[test]
    fn identical_parent_crossover_keeps_dna_but_consumes_eager_per_block_draws() {
        let dna = LegacyDna::parse("start 42 900 store stop").unwrap();
        let mut random = crate::native_random::NativeRandom::default();
        let mut expected = random.clone();
        // One common run, including dummy zero and master end.
        for _ in 0..dna.blocks.len() + 2 { expected.unit(); }
        let child = dna.crossover_native(&dna, &mut random);
        assert_eq!(child.blocks(), dna.blocks());
        assert_eq!(random, expected);
    }

    #[test]
    fn insertion_alignment_does_not_cut_common_tail_in_half_and_resumes_exactly() {
        let left = LegacyDna::parse("start 42 900 store 17 901 store stop").unwrap();
        let right = LegacyDna::parse("start 99 902 store 42 900 store 17 901 store stop").unwrap();
        let mut random = crate::native_random::NativeRandom::with_user_seed(2468);
        let saved = serde_json::to_string(&random).unwrap();
        let mut restored = serde_json::from_str(&saved).unwrap();
        let child = left.crossover_native(&right, &mut random);
        let resumed = left.crossover_native(&right, &mut restored);
        assert_eq!(child.blocks(), resumed.blocks()); assert_eq!(random, restored);
        assert!(child.blocks().windows(3).any(|blocks| blocks == &[
            DnaBlock { class: TokenClass::Number, value: 17 },
            DnaBlock { class: TokenClass::Number, value: 901 },
            DnaBlock { class: TokenClass::Store, value: 1 },
        ]));
        assert_eq!(child.blocks().last().unwrap().class, TokenClass::End);
    }
}
