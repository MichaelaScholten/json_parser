use std::{
    env::args,
    fs::File,
    io::{BufReader, Read},
};

use json_parser::Json;

fn main() {
    Json::from_bytes(
        BufReader::new(
            File::open(args().nth(1).expect("Expected a filepath as argument")).unwrap(),
        )
        .bytes()
        .map_while(Result::ok),
    )
    .unwrap();
}
