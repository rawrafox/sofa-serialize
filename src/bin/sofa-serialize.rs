extern crate rustc_serialize;
extern crate sofa_serialize;

use std::env;
use std::fs;
use std::io;
use std::io::{Read};

use sofa_serialize::Serialize;

fn read_dictionary(name: &str) -> io::Result<Vec<String>> {
    let mut r = String::new();
    let mut f = try!(fs::File::open(name));
    try!(f.read_to_string(&mut r));
    return Ok(r.lines().map(|x| { x.to_string() }).collect());
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let dictionary = read_dictionary(&args[1]).unwrap();

    let (mut stdin, mut stdout) = (io::stdin(), io::stdout());
    let json = rustc_serialize::json::Json::from_reader(&mut stdin).unwrap();

    let mut encoder = sofa_serialize::Encoder::new(&mut stdout, dictionary);

    json.serialize(&mut encoder).unwrap();
}
