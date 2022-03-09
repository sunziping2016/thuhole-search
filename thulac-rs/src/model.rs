use ndarray::Array2;
use std::{
    io::{Read, Result},
    iter,
    mem::size_of,
    slice, usize,
};

use crate::{Dat, Label, Poc};

const SENTENCE_BOUNDARY: char = '#';
const FEATURE_SEPARATOR: char = ' ';
const FEATURE_UNI_L: char = '2';
const FEATURE_UNI_M: char = '1';
const FEATURE_UNI_R: char = '3';
const FEATURE_BI_LL: char = '3';
const FEATURE_BI_LM: char = '1';
const FEATURE_BI_MR: char = '2';
const FEATURE_BI_RR: char = '4';

#[allow(dead_code)]
mod test_endianness {
    use byteorder::{LittleEndian, NativeEndian};

    struct Unit<T>(T);
    fn test_endianness(endianness: Unit<NativeEndian>) {
        match endianness {
            Unit::<LittleEndian>(_) => (),
        }
    }
}

pub struct Model {
    ll_weights: Array2<i32>,
    fl_weights: Array2<i32>,
}

pub fn normalize_char(ch: char) -> char {
    let ord = ch as u32;
    if ord > 32 && ord < 128 {
        unsafe { char::from_u32_unchecked(ord + 65248) }
    } else {
        ch
    }
}

impl Model {
    pub fn load<R: Read>(reader: &mut R) -> Result<Model> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let l_size = u32::from_le_bytes(buf) as usize;
        reader.read_exact(&mut buf)?;
        let f_size = u32::from_le_bytes(buf) as usize;
        let mut ll_weights = Array2::zeros((l_size, l_size));
        let mut fl_weights = Array2::zeros((f_size, l_size));
        let ll_slice = ll_weights.as_slice_mut().unwrap();
        let fl_slice = fl_weights.as_slice_mut().unwrap();
        reader.read_exact(unsafe {
            slice::from_raw_parts_mut(
                ll_slice.as_mut_ptr() as *mut u8,
                ll_slice.len() * size_of::<i32>(),
            )
        })?;
        reader.read_exact(unsafe {
            slice::from_raw_parts_mut(
                fl_slice.as_mut_ptr() as *mut u8,
                fl_slice.len() * size_of::<i32>(),
            )
        })?;
        Ok(Model {
            ll_weights,
            fl_weights,
        })
    }

    pub fn init_scores(&self, dat: &Dat, sentence: &str, sentence_len: usize) -> Array2<i32> {
        let b = SENTENCE_BOUNDARY;
        let f = FEATURE_SEPARATOR;
        let mut chars = sentence
            .chars()
            .chain(iter::once(b))
            .chain(iter::once(b))
            .map(normalize_char);
        let ch_m = chars.next().unwrap();
        let ch_r = chars.next().unwrap();
        let base_l = dat.child(dat.root(), b);
        let base_m = dat.child(dat.root(), ch_m);
        let mut base_r = dat.child(dat.root(), ch_r);
        let mut uni_l = dat.and_child(base_l, f);
        let mut uni_m = dat.and_child(base_m, f);
        let mut uni_r = dat.and_child(base_r, f);
        let mut bi_ll = dat.and_child(dat.and_child(base_l, b), f);
        let mut bi_lm = dat.and_child(dat.and_child(base_l, ch_m), f);
        let mut bi_mr = dat.and_child(dat.and_child(base_m, ch_r), f);
        let mut scores = Array2::<i32>::zeros((sentence_len, self.fl_weights.ncols()));
        for (i, ch) in chars.enumerate() {
            let mut score = scores.row_mut(i);
            let base_rr = dat.child(dat.root(), ch);
            let uni_rr = dat.and_child(base_rr, f);
            let bi_rr = dat.and_child(dat.and_child(base_r, ch), f);
            if let Some(x) = dat.and_child(uni_l, FEATURE_UNI_L) {
                score += &self.fl_weights.row(dat.base(x) as usize);
            }
            if let Some(x) = dat.and_child(uni_m, FEATURE_UNI_M) {
                score += &self.fl_weights.row(dat.base(x) as usize);
            }
            if let Some(x) = dat.and_child(uni_r, FEATURE_UNI_R) {
                score += &self.fl_weights.row(dat.base(x) as usize);
            }
            if let Some(x) = dat.and_child(bi_ll, FEATURE_BI_LL) {
                score += &self.fl_weights.row(dat.base(x) as usize);
            }
            if let Some(x) = dat.and_child(bi_lm, FEATURE_BI_LM) {
                score += &self.fl_weights.row(dat.base(x) as usize);
            }
            if let Some(x) = dat.and_child(bi_mr, FEATURE_BI_MR) {
                score += &self.fl_weights.row(dat.base(x) as usize);
            }
            if let Some(x) = dat.and_child(bi_rr, FEATURE_BI_RR) {
                score += &self.fl_weights.row(dat.base(x) as usize);
            }
            base_r = base_rr;
            uni_l = uni_m;
            uni_m = uni_r;
            uni_r = uni_rr;
            bi_ll = bi_lm;
            bi_lm = bi_mr;
            bi_mr = bi_rr;
        }
        scores
    }

    pub fn decode(
        &self,
        scores: &mut Array2<i32>,
        pocs: &[Poc],
        label: &Label,
    ) -> Option<Vec<usize>> {
        assert_eq!(scores.nrows(), pocs.len());
        if pocs.is_empty() {
            return Some(Vec::new());
        }
        let mut prev = Array2::<usize>::from_elem((scores.nrows(), scores.ncols()), usize::MAX);
        label
            .allowed_labels(*pocs.first().unwrap())
            .iter()
            .copied()
            .for_each(|j| prev[[0, j]] = usize::MAX - 1);
        for (i, poc) in pocs.iter().copied().enumerate().skip(1) {
            let prev_i = i - 1;
            for j in label.allowed_labels(poc).iter().copied() {
                let mut best_j = 0;
                let mut best_score = i32::MIN;
                for prev_j in label.prev_labels(j).iter().copied() {
                    if prev[[prev_i, prev_j]] == usize::MAX {
                        continue;
                    }
                    let score = scores[[prev_i, prev_j]] + self.ll_weights[[prev_j, j]];
                    if score > best_score {
                        best_j = prev_j;
                        best_score = score;
                    }
                }
                scores[[i, j]] += best_score;
                prev[[i, j]] = best_j;
            }
        }
        let last_row = prev.nrows() - 1;
        if let Some(mut last) = (0..prev.ncols())
            .filter(|&j| prev[[last_row, j]] != usize::MAX)
            .max_by_key(|&j| scores[[last_row, j]])
        {
            let mut answer = Vec::with_capacity(prev.nrows());
            answer.push(last);
            for i in (1..prev.nrows()).rev() {
                last = prev[[i, last]];
                answer.push(last);
            }
            answer.reverse();
            Some(answer)
        } else {
            None
        }
    }
}
