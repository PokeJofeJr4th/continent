use std::{fs, io::Write};

#[path = "src/mkv.rs"]
mod mkv;

use mkv::MarkovData;

#[warn(clippy::pedantic)]

fn main() {
    println!("cargo:rerun-if-changed=markov/");
    println!("cargo:rerun-if-changed=csv/");
    println!("cargo:rerun-if-changed=build.rs");

    macro_rules! markov_data {
        {$($src_path: expr => $dest_path: expr),*} => {
            $(
                println!($src_path);
                let markov_data: MarkovData = MarkovData::from_csv($src_path).unwrap();
                println!($dest_path);
                let mut f = fs::File::create($dest_path).unwrap();
                f.write_all(&markov_data.to_bytes()).unwrap();
            )*
        }
    }

    markov_data! {
        "csv/animal.csv" => "markov/animal.mkv",
        "csv/gemstone.csv" => "markov/gemstone.mkv",
        "csv/magic.csv" => "markov/magic.mkv",
        "csv/metal.csv" => "markov/metal.mkv",
        "csv/monster.csv" => "markov/monster.mkv",
        "csv/name.csv" => "markov/name.mkv",
        "csv/plant.csv" => "markov/plant.mkv"
    }
}
