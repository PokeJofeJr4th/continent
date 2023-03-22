use std::{collections::HashMap, fs, io::Write};
use rand::distributions::WeightedIndex;


pub type MarkovData = (
    Vec<(char, char)>,
    HashMap<(char, char), (Vec<char>, Vec<u32>, WeightedIndex<u32>)>,
);

pub fn markov_from_file(filename: &str) -> Option<MarkovData> {
    let string_data: Vec<String> = match fs::read_to_string(filename) {
        Ok(res) => res,
        Err(_) => return None,
    }
    .split(',')
    .map(|string| string.to_lowercase() + ";")
    .collect();
    Some(get_markov_data(
        &string_data
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&str>>(),
    ))
}

pub fn get_markov_data(strings: &[&str]) -> MarkovData {
    {
        let mut counts: HashMap<((char, char), char), u32> = HashMap::new();
        let mut starts = Vec::new();
        for string in strings {
            starts.push({
                let mut chars = string.chars();
                (
                    match chars.next() {
                        Some(res) => res,
                        None => continue,
                    },
                    match chars.next() {
                        Some(res) => res,
                        None => continue,
                    },
                )
            });
            for i in 0..(string.len() - 2) {
                let mut chars = string.chars();
                let char_triple = (
                    (
                        match chars.nth(i) {
                            Some(c) => c,
                            None => continue,
                        },
                        match chars.next() {
                            Some(c) => c,
                            None => continue,
                        },
                    ),
                    match chars.next() {
                        Some(c) => c,
                        None => continue,
                    },
                );
                counts.insert(char_triple, {
                    match counts.get(&char_triple) {
                        Some(c) => c + 1,
                        None => 1,
                    }
                });
            }
        }
        let mut intermediate_counts: HashMap<(char, char), (Vec<char>, Vec<u32>)> = HashMap::new();
        for (&(k, character), &amount) in counts.iter() {
            intermediate_counts.insert(k, {
                let mut vectors = match intermediate_counts.get(&k) {
                    Some(vecs) => vecs.clone(),
                    None => (Vec::new(), Vec::new()),
                };
                vectors.0.push(character);
                vectors.1.push(amount);
                vectors
            });
        }
        (
            starts,
            intermediate_counts
                .iter()
                .filter_map(|(&k, (chars, weights))| match WeightedIndex::new(weights) {
                    Ok(res) => Some((k, (chars.clone(), weights.clone(), res))),
                    Err(_) => None,
                })
                .collect(),
        )
    }
}

// first 3 bits are count. last 5 are letter
fn char_to_byte((char, weight): (char, u8)) -> Option<u8> {
    let char_part: u8;
    if char == ';' {
        char_part = 27;
    } else if char as u8 > 96 && (char as u8) < 123 {
        char_part = char as u8 - 95;
    } else {
        println!("{} as u8 = {}", char, char as u8);
        return None;
    }
    Some((weight.clamp(1, 8) - 1) * 32 + char_part)
}

pub fn markov_to_bytes(markov_data: &MarkovData) -> Vec<u8> {
    let mut bytes = Vec::new();
    for &(char1, char2) in &markov_data.0 {
        bytes.push(char_to_byte((char1, 0)).unwrap());
        bytes.push(char_to_byte((char2, 0)).unwrap());
    }
    bytes.push(0);
    for (&(char1, char2), (characters, weights, _index)) in &markov_data.1 {
        bytes.push(char_to_byte((char1, 0)).unwrap());
        bytes.push(char_to_byte((char2, 0)).unwrap());
        for character_index in 0..characters.len() {
            assert!(character_index < characters.len());
            bytes.push(
                char_to_byte((characters[character_index], weights[character_index] as u8))
                    .unwrap(),
            );
        }
        bytes.push(0);
    }
    bytes
}

fn main() {
    macro_rules! markov_data {
        {$($src_path: expr => $dest_path: expr),*} => {
            $(let markov_data: MarkovData = markov_from_file($src_path).unwrap();
            let mut f = fs::File::create($dest_path).unwrap();
            f.write_all(&markov_to_bytes(&markov_data)).unwrap();)*
        }
    }

    markov_data!{
        "csv/animal.csv" => "markov/animal.markov",
        "csv/gemstone.csv" => "markov/gemstone.markov",
        "csv/magic.csv" => "markov/magic.markov",
        "csv/metal.csv" => "markov/metal.markov",
        "csv/monster.csv" => "markov/monster.markov",
        "csv/name.csv" => "markov/name.markov",
        "csv/plant.csv" => "markov/plant.markov"
    }
}
