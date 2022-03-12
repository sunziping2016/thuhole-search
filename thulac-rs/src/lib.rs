#![feature(char_indices_offset)]
#![feature(seek_stream_len)]
#![feature(result_flattening)]

mod dat;
mod label;
mod model;
mod poc;
mod post;
mod t2s;

use std::fs::File;
use std::io::{BufReader, ErrorKind, Result};
use std::ops::Range;
use std::path::Path;

pub use dat::Dat;
pub use label::Label;
pub use model::Model;
pub use poc::punc_adjust;
pub use poc::Poc;
pub use post::PostProcessor;
pub use t2s::T2S;

pub struct Thulac {
    label: Label,
    model: Model,
    dat: Dat,
    t2s: Option<T2S>,
    posts: Vec<PostProcessor>,
}

pub struct Preprocess<'a> {
    raw: &'a str,
    input: String,
    pocs: Vec<Poc>,
}

impl Thulac {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let label = Label::load(&mut BufReader::new(File::open(path.join("label.txt"))?))?;
        let model = Model::load(&mut File::open(path.join("model.bin"))?)?;
        let dat = Dat::load(&mut File::open(path.join("dat.bin"))?)?;
        let t2s = File::open(path.join("t2s.bin"))
            .map(|mut x| T2S::load(&mut x))
            .flatten()
            .map(Option::Some)
            .or_else(|e| {
                if e.kind() == ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(e)
                }
            })?;
        let mut posts = Vec::new();
        for (name, tag) in [("ns.bin", "ns"), ("idiom.bin", "i")] {
            match File::open(path.join(name)) {
                Ok(mut file) => {
                    posts.push(PostProcessor::new(Dat::load(&mut file)?, tag.into()));
                }
                Err(e) if e.kind() == ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
        }
        Ok(Self {
            label,
            model,
            dat,
            t2s,
            posts,
        })
    }
    pub fn add_postprocessor(&mut self, post: PostProcessor) {
        self.posts.push(post);
    }
    pub fn preprocess<'a>(&self, raw: &'a str) -> Preprocess<'a> {
        let (input, pocs) = if let Some(t2s) = self.t2s.as_ref() {
            Poc::build(t2s.process(raw.chars()))
        } else {
            Poc::build(raw.chars())
        };
        Preprocess { raw, input, pocs }
    }
    pub fn cut<'a, 'b>(
        &'a self,
        preprocess: &'b Preprocess<'_>,
    ) -> Vec<(Range<usize>, &'b str, &'a str)> {
        let Preprocess { raw, input, pocs } = preprocess;
        let mut scores = self.model.init_scores(&self.dat, input, pocs.len());
        let path = self
            .model
            .decode(&mut scores, pocs, &self.label)
            .expect("failed to segment");
        let mut last_raw = 0;
        let mut last_input = 0;
        let mut input_chars = input.char_indices();
        let (mut words, raw_chars) = path.iter().copied().fold(
            (Vec::<(_, &'b str, &'a str)>::new(), raw.char_indices()),
            move |(mut words, mut raw_chars), i| {
                let (poc, desc) = self.label.label(i);
                if raw_chars.next().unwrap().1.is_whitespace() {
                    assert!(matches!(*poc, Poc::B | Poc::S));
                    loop {
                        let (next, next_ch) = raw_chars.next().unwrap();
                        if !next_ch.is_whitespace() {
                            words.push((last_raw..next, "", "w"));
                            last_raw = next;
                            break;
                        }
                    }
                }
                let _ = input_chars.next();
                if matches!(*poc, Poc::E | Poc::S) {
                    words.push((
                        last_raw..raw_chars.offset(),
                        &input[last_input..input_chars.offset()],
                        &desc[..],
                    ));
                    last_raw = raw_chars.offset();
                    last_input = input_chars.offset();
                }
                (words, raw_chars)
            },
        );
        if raw_chars.offset() != raw.len() {
            words.push((raw_chars.offset()..raw.len(), "", "w"));
        }
        for post in self.posts.iter() {
            words = post.adjust(words);
        }
        words
    }
}
