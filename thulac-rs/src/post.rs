use core::slice;

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
    pub fn adjust<'a, 'b>(&'b self, mut words: Vec<(&'a str, &'b str)>) -> Vec<(&'a str, &'b str)> {
        words.reverse();
        let mut result = Vec::new();
        while let Some((word, tag)) = words.pop() {
            if let Some(mut pointer) = self.dat.descendant(self.dat.root(), word) {
                let mut best = self.dat.child(pointer, '\0').map(|_| words.len());
                for (index, (word, _)) in words.iter().enumerate().rev() {
                    match self.dat.descendant(pointer, word) {
                        Some(new_pointer) => {
                            pointer = new_pointer;
                            best = self.dat.child(pointer, '\0').map(|_| index).or(best);
                        }
                        None => break,
                    }
                }
                if let Some(best) = best {
                    let word = words
                        .drain(best..words.len())
                        .rev()
                        .fold(word, |acc, (word, _)| unsafe { concat_str(acc, word) });
                    result.push((word, &self.tag[..]));
                    continue;
                }
            }
            result.push((word, tag));
        }
        result
    }
}
