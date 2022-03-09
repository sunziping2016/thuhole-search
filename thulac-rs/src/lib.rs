#![feature(char_indices_offset)]
#![feature(seek_stream_len)]

mod dat;
mod label;
mod model;
mod poc;
mod post;
mod t2s;

pub use dat::Dat;
pub use label::Label;
pub use model::Model;
pub use poc::punc_adjust;
pub use poc::Poc;
pub use post::PostProcessor;
pub use t2s::T2S;

pub fn segment<'a, 'b>(line: &'a str, path: &[usize], label: &'b Label) -> Vec<(&'a str, &'b str)> {
    let mut last = 0;
    let (mut words, rest) = path.iter().copied().fold(
        (Vec::new(), line.char_indices()),
        move |(mut words, mut chars), i| {
            let (poc, desc) = label.label(i);
            if chars.next().unwrap().1.is_whitespace() {
                assert!(matches!(*poc, Poc::B | Poc::S));
                loop {
                    let (next, next_ch) = chars.next().unwrap();
                    if !next_ch.is_whitespace() {
                        words.push((&line[last..next], "w"));
                        last = next;
                        break;
                    }
                }
            }
            if matches!(*poc, Poc::E | Poc::S) {
                words.push((&line[last..chars.offset()], &desc[..]));
                last = chars.offset();
            }
            (words, chars)
        },
    );
    let rest = rest.as_str();
    if !rest.is_empty() {
        words.push((rest, "w"));
    }
    words
}
