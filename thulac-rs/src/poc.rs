use bitflags::bitflags;
use ctor::ctor;
use phf::{phf_set, Set};

bitflags! {
    pub struct Poc: u8 {
        const B = 0x1;
        const M = 0x2;
        const E = 0x4;
        const S = 0x8;

        const BS = Self::B.bits | Self::S.bits;
        const ES = Self::E.bits | Self::S.bits;
        const ANY = Self::B.bits | Self::M.bits | Self::E.bits | Self::S.bits;
    }
}

pub static SINGLE_PUNC: Set<char> = phf_set! {
    '，', '。', '？', '！', '：', '；', '‘', '’', '“', '”', '【', '】', '、', '《', '》',
    '（', '）', ',', '.', '?', '!', ';', ':', '\'', '"', '(', ')',
};

pub static MULTI_PUNC: Set<char> = phf_set! {
    '·', '@', '|', '#', '￥', '%', '…', '&', '*', '—', '-', '+', '=', '<', '>', '/', '{', '}',
    '[', ']', '\\', '$', '^', '_', '`', '~',
};

#[ctor]
fn check_disjoint() {
    assert!(SINGLE_PUNC.is_disjoint(&MULTI_PUNC));
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CharKind {
    Space,
    SinglePunc,
    MultiPunc,
    Others,
}

impl CharKind {
    pub fn find(ch: char) -> Self {
        if ch.is_whitespace() {
            CharKind::Space
        } else if SINGLE_PUNC.contains(&ch) {
            CharKind::SinglePunc
        } else if ch.is_ascii_alphanumeric() || MULTI_PUNC.contains(&ch) {
            CharKind::MultiPunc
        } else {
            CharKind::Others
        }
    }
}

impl Poc {
    pub fn build<S: IntoIterator<Item = char>>(sentence: S) -> (String, Vec<Poc>) {
        let mut curr = CharKind::Space;
        let mut result = sentence.into_iter().fold(
            (String::new(), Vec::<Poc>::new()),
            move |(mut result, mut pocs), ch| {
                let prev = curr;
                curr = CharKind::find(ch);
                let last = pocs.last_mut().into_iter();
                match (prev, curr) {
                    (CharKind::MultiPunc, CharKind::MultiPunc) => {
                        last.for_each(|x| *x &= Poc::B | Poc::M);
                        pocs.push(Poc::M | Poc::E);
                        result.push(ch);
                    }
                    (CharKind::Others, CharKind::Others) => {
                        pocs.push(Poc::ANY);
                        result.push(ch);
                    }
                    (_, curr) => {
                        last.for_each(|x| *x &= Poc::ES);
                        match curr {
                            CharKind::Space => (),
                            CharKind::SinglePunc => {
                                pocs.push(Poc::S);
                                result.push(ch);
                            }
                            CharKind::MultiPunc | CharKind::Others => {
                                pocs.push(Poc::BS);
                                result.push(ch);
                            }
                        }
                    }
                }
                (result, pocs)
            },
        );
        result.1.last_mut().into_iter().for_each(|x| *x &= Poc::ES);
        assert!(result.1.iter().all(|&x| x == Poc::B
            || x == Poc::M
            || x == Poc::E
            || x == Poc::S
            || x == Poc::BS
            || x == Poc::ES
            || x == Poc::ANY));
        (result.0, result.1)
    }
}

pub fn punc_adjust<'a, 'b>(mut words: Vec<(&'a str, &'b str)>) -> Vec<(&'a str, &'b str)> {
    words.iter_mut().for_each(|word| {
        let mut chars = word.0.chars();
        let ch = chars.next().unwrap();
        if chars.next().is_none() && SINGLE_PUNC.contains(&ch) {
            word.1 = "w";
        }
    });
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poc() {
        assert_eq!(Poc::build("".chars()), ("".to_owned(), vec![]));
        assert_eq!(Poc::build(" ".chars()), ("".to_owned(), vec![]));
        assert_eq!(Poc::build(".".chars()), (".".to_owned(), vec![Poc::S]));
        assert_eq!(Poc::build("h".chars()), ("h".to_owned(), vec![Poc::S]));
        assert_eq!(Poc::build("我".chars()), ("我".to_owned(), vec![Poc::S]));
        assert_eq!(
            Poc::build("hey, 你好呀！".chars()),
            (
                "hey,你好呀！".to_owned(),
                vec![
                    Poc::B,
                    Poc::M,
                    Poc::E,
                    Poc::S,
                    Poc::BS,
                    Poc::ANY,
                    Poc::ES,
                    Poc::S,
                ]
            )
        );
    }
}
