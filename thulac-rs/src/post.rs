use core::slice;
use std::ops::Range;

use crate::Dat;

pub struct PostProcessor {
    dat: Dat,
    tag: String,
}

unsafe fn concat_slice<'a, T>(a: &'a [T], b: &'a [T]) -> &'a [T] {
    assert_eq!(a.as_ptr().add(a.len()), b.as_ptr());
    slice::from_raw_parts(a.as_ptr(), a.len() + b.len())
}

unsafe fn concat_str<'a>(a: &'a str, b: &'a str) -> &'a str {
    std::str::from_utf8_unchecked(concat_slice(a.as_bytes(), b.as_bytes()))
}

impl PostProcessor {
    pub fn new(dat: Dat, tag: String) -> Self {
        Self { dat, tag }
    }
    pub fn adjust<'a, 'b>(
        &'b self,
        mut words: Vec<(Range<usize>, &'a str, &'b str)>,
    ) -> Vec<(Range<usize>, &'a str, &'b str)> {
        words.reverse();
        let mut result = Vec::new();
        while let Some((range, word, tag)) = words.pop() {
            if !word.is_empty() {
                if let Some(mut pointer) = self.dat.descendant(self.dat.root(), word) {
                    let mut best = self.dat.child(pointer, '\0').map(|_| words.len());
                    for (index, (_, word, _)) in words.iter().enumerate().rev() {
                        if word.is_empty() {
                            break;
                        }
                        match self.dat.descendant(pointer, word) {
                            Some(new_pointer) => {
                                pointer = new_pointer;
                                best = self.dat.child(pointer, '\0').map(|_| index).or(best);
                            }
                            None => break,
                        }
                    }
                    if let Some(best) = best {
                        let (range, word) = words.drain(best..words.len()).rev().fold(
                            (range, word),
                            |(acc_range, acc_word), (range, word, _)| {
                                assert!(acc_range.end == range.start);
                                (acc_range.start..range.end, unsafe {
                                    concat_str(acc_word, word)
                                })
                            },
                        );
                        result.push((range, word, &self.tag[..]));
                        continue;
                    }
                }
            }
            result.push((range, word, tag));
        }
        result
    }
}
