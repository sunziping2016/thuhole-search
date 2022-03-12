#![feature(stdin_forwarders)]

use std::{env, io::stdin, path::PathBuf, str::FromStr};

use thulac_rs::Thulac;

fn main() {
    let path = PathBuf::from_str(
        &env::var("THULAC_MODEL_PATH").expect("failed to fetch env THULAC_MODEL_PATH"),
    )
    .expect("invalid path");
    let thulac = Thulac::load(&path).expect("failed to load model");
    for line in stdin().lines() {
        let line = line.expect("failed to read line");
        let preprocess = thulac.preprocess(&line);
        let result = thulac.cut(&preprocess);
        println!("{:?}", result);
    }
}
