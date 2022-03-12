#![feature(stdin_forwarders)]
#![feature(result_flattening)]

use std::{
    env,
    fs::File,
    io::{stdin, BufReader, ErrorKind},
    path::PathBuf,
    str::FromStr,
};

use thulac_rs::{Dat, PostProcessor, Thulac};

fn main() {
    let path = PathBuf::from_str(
        &env::var("THULAC_MODEL_PATH").expect("failed to fetch env THULAC_MODEL_PATH"),
    )
    .expect("invalid path");
    let mut thulac = Thulac::load(&path).expect("failed to load model");
    match File::open("user.txt")
        .map(|x| Dat::load_set_txt(&mut BufReader::new(x), true))
        .flatten()
    {
        Ok(dat) => {
            thulac.add_postprocessor(PostProcessor::new(dat, "uw".into()));
        }
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(e) => panic!("{}", e),
    }
    for line in stdin().lines() {
        let line = line.expect("failed to read line");
        let preprocess = thulac.preprocess(&line);
        let result = thulac.cut(&preprocess);
        println!("{:?}", result);
    }
}
