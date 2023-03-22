use std::{collections::HashMap, fs};

use rand::{distributions::WeightedIndex, rngs::ThreadRng, seq::SliceRandom, prelude::Distribution};

#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
pub(crate) struct MarkovData {
    starts: Vec<(char, char)>,
    map: HashMap<(char, char), (Vec<char>, Vec<u32>, WeightedIndex<u32>)>
}

// for some reason it thinks this is dead even though it isn't
#[allow(dead_code)]
impl MarkovData {
    pub fn sample(&self, rng: &mut ThreadRng) -> String {
        loop {
            if let Some(res) = self.try_sample(rng) {
                return res;
            }
        }
    }

    pub fn try_sample(&self, rng: &mut ThreadRng) -> Option<String> {
        let mut result: String = {
            let chars: (char, char) = match self.starts.choose(rng) {
                Some(&res) => res,
                None => return None,
            };
            let mut string = String::new();
            string.push(chars.0);
            string.push(chars.1);
            string
        };
        loop {
            let ending = {
                let mut chars = result.chars();
                (
                    match chars.nth(result.len() - 2) {
                        Some(res) => res,
                        None => break,
                    },
                    match chars.next() {
                        Some(res) => res,
                        None => break,
                    },
                )
            };
            result.push(match self.map.get(&ending) {
                Some(result) => match result.0.get(result.2.sample(rng)) {
                    Some(&';') => break,
                    Some(&c) => c,
                    None => break,
                },
                None => break,
            })
        }
        // println!("{result:?}");
        if 5 < result.len() && result.len() < 15 {
            Some(
                result
                    .split(' ')
                    .map(|word| {
                        let mut chars = word.chars();
                        (match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        } + " ")
                    })
                    .collect::<String>(),
            )
        } else {
            None
        }
    }

    pub fn from_csv(filename: &str) -> Option<Self> {
        let string_data: Vec<String> = match fs::read_to_string(filename) {
            Ok(res) => res,
            Err(_) => return None,
        }
        .split(',')
        .map(|string| string.to_lowercase() + ";")
        .collect();
        Some(Self::from_strings(
            &string_data
                .iter()
                .map(|s| s.as_ref())
                .collect::<Vec<&str>>(),
        ))
    }

    pub fn from_strings(strings: &[&str]) -> Self {
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
            MarkovData{
                starts,
                map: intermediate_counts
                    .iter()
                    .filter_map(|(&k, (chars, weights))| match WeightedIndex::new(weights) {
                        Ok(res) => Some((k, (chars.clone(), weights.clone(), res))),
                        Err(_) => None,
                    })
                    .collect(),
                }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for &(char1, char2) in &self.starts {
            bytes.push(char_to_byte((char1, 0)).unwrap());
            bytes.push(char_to_byte((char2, 0)).unwrap());
        }
        bytes.push(0);
        for (&(char1, char2), (characters, weights, _index)) in &self.map {
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

    pub fn from_bytes(bytes: Vec<u8>) -> Option<Self> {
        let mut bytes_iter = bytes.iter();
        let mut starts = Vec::new();
        loop {
            match bytes_iter.next() {
                None => {
                    println!("bytes were empty");
                    return None;
                }
                Some(0) => break,
                Some(&first_byte) => starts.push((
                    byte_to_char(first_byte).unwrap().0,
                    byte_to_char(*bytes_iter.next().unwrap()).unwrap().0,
                )),
            }
        }
        let mut intermediate_counts: HashMap<(char, char), (Vec<char>, Vec<u32>)> = HashMap::new();
        // over each pair of characters that can end the word
        loop {
            let char_pair = match bytes_iter.next() {
                None => break,
                Some(0) => break,
                Some(&first_byte) => (
                    byte_to_char(first_byte).unwrap().0,
                    byte_to_char(*bytes_iter.next().unwrap()).unwrap().0,
                ),
            };
            let mut weights: Vec<(char, u32)> = Vec::new();
            // over each possible character that could come next
            loop {
                match bytes_iter.next() {
                    None => break,
                    Some(0) => break,
                    Some(&val) => {
                        weights.push(byte_to_char(val).unwrap());
                    }
                }
            }
            intermediate_counts.insert(char_pair, weights.iter().copied().unzip());
        }
        Some(Self{
            starts,
            map: intermediate_counts
                .iter()
                .filter_map(|(&k, (chars, weights))| match WeightedIndex::new(weights) {
                    Ok(res) => Some((k, (chars.clone(), weights.clone(), res))),
                    Err(_) => None,
                })
                .collect(),
    })
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

fn byte_to_char(byte: u8) -> Option<(char, u32)> {
    match 27.cmp(&(byte % 32)) {
        std::cmp::Ordering::Less => {
            println!("{} - 31 as char = {}", byte, (byte - 31) as char);
            None
        }
        std::cmp::Ordering::Equal => Some((';', (byte / 32 + 1) as u32)),
        std::cmp::Ordering::Greater => Some(((byte % 32 + 95) as char, (byte / 32 + 1) as u32)),
    }
}