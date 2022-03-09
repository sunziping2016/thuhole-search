use std::{
    io::{BufRead, Result},
    usize,
};

use crate::Poc;

pub struct Label {
    labels: Vec<(Poc, String)>,
    poc2label: [Vec<usize>; 16],
    prev_labels: Vec<Vec<usize>>,
}

impl Label {
    pub fn load<R: BufRead>(reader: &mut R) -> Result<Self> {
        let mut labels = reader.lines().collect::<Result<Vec<String>>>()?;
        while let Some(true) = labels.last().map(|x| x.is_empty()) {
            labels.pop();
        }
        let labels = labels
            .into_iter()
            .map(|x| {
                let mut chars = x.chars();
                let poc = match chars.next().expect("empty line in labels") {
                    '0' => Poc::B,
                    '1' => Poc::M,
                    '2' => Poc::E,
                    '3' => Poc::S,
                    _ => panic!("unknown poc in labels"),
                };
                (poc, chars.as_str().to_string())
            })
            .collect::<Vec<_>>();
        let mut poc2label: [Vec<usize>; 16] = Default::default();
        labels.iter().enumerate().for_each(|(i, &(poc, _))| {
            for j in 0..16 {
                if j & poc.bits() != 0 {
                    poc2label[j as usize].push(i);
                }
            }
        });
        let prev_labels = labels
            .iter()
            .map(|curr| {
                labels
                    .iter()
                    .enumerate()
                    .filter_map(|(i, prev)| {
                        if matches!(prev.0, Poc::E | Poc::S) && matches!(curr.0, Poc::B | Poc::S) {
                            return Some(i);
                        }
                        if prev.1 == curr.1
                            && matches!(prev.0, Poc::B | Poc::M)
                            && matches!(curr.0, Poc::M | Poc::E)
                        {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        Ok(Label {
            labels,
            poc2label,
            prev_labels,
        })
    }

    pub fn label(&self, index: usize) -> &(Poc, String) {
        &self.labels[index]
    }

    pub fn allowed_labels(&self, poc: Poc) -> &[usize] {
        &self.poc2label[poc.bits() as usize]
    }

    pub fn prev_labels(&self, index: usize) -> &[usize] {
        &self.prev_labels[index]
    }
}
