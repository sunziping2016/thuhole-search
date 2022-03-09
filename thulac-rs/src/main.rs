#![feature(stdin_forwarders)]

use std::{
    env,
    fs::File,
    io::{stdin, BufReader},
};

use thulac_rs::{punc_adjust, segment, Dat, Label, Model, Poc, PostProcessor, T2S};

fn option_file_from_env(var: &str) -> Option<File> {
    match env::var(var) {
        Ok(filename) => {
            Some(File::open(&filename).unwrap_or_else(|_| panic!("failed to open {}", filename)))
        }
        Err(env::VarError::NotPresent) => None,
        _ => panic!("invalid encoding of env {}", var),
    }
}

fn file_from_env(var: &str) -> File {
    option_file_from_env(var)
        .unwrap_or_else(|| panic!("failed to open file referred by env {}", var))
}

fn main() {
    let label = Label::load(&mut BufReader::new(file_from_env("THULAC_LABEL")))
        .expect("failed to parse file refer by THULAC_LABEL");
    let model = Model::load(&mut file_from_env("THULAC_MODEL"))
        .expect("failed to parse file refer by THULAC_MODEL");
    let dat = Dat::load(&mut file_from_env("THULAC_DAT"))
        .expect("failed to parse file refer by THULAC_DAT");
    let ns = option_file_from_env("THULAC_NS").map(|mut f| {
        PostProcessor::new(
            Dat::load(&mut f).expect("failed to parse file referred by THULAC_NS"),
            "ns".into(),
        )
    });
    let idiom = option_file_from_env("THULAC_IDIOM").map(|mut f| {
        PostProcessor::new(
            Dat::load(&mut f).expect("failed to parse file referred by THULAC_IDIOM"),
            "i".into(),
        )
    });
    let t2s = option_file_from_env("THULAC_T2S")
        .map(|mut f| T2S::load(&mut f).expect("failed to parse file referred by THULAC_T2S"));
    for line in stdin().lines() {
        let raw_line = line.expect("failed to read from stdin");
        let (line, pocs) = if let Some(t2s) = t2s.as_ref() {
            Poc::build(t2s.process(raw_line.chars()))
        } else {
            Poc::build(raw_line.chars())
        };
        let mut scores = model.init_scores(&dat, &line, pocs.len());
        let path = model
            .decode(&mut scores, &pocs, &label)
            .expect("failed to segment");
        let mut words = segment(&raw_line, &path, &label);
        if let Some(ns) = ns.as_ref() {
            words = ns.adjust(words);
        }
        if let Some(idiom) = idiom.as_ref() {
            words = idiom.adjust(words);
        }
        words = punc_adjust(words);
        println!("{:?}", words);
    }
}

// fn main() {
//     let arg = env::args().collect::<Vec<_>>();
//     let dat = Dat::load_map_txt(&mut BufReader::new(File::open(&arg[1]).unwrap()), false).unwrap();
//     dat.save(&mut File::create(&arg[2]).unwrap()).unwrap();
// }
