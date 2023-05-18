use std::{collections::HashMap, fs};

use rand::{
    distributions::WeightedIndex, prelude::Distribution, rngs::ThreadRng, seq::SliceRandom,
};

#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
pub struct MarkovData {
    starts: Vec<(char, char)>,
    map: HashMap<(char, char), (Vec<char>, Vec<u32>, WeightedIndex<u32>)>,
}

#[allow(dead_code)]
pub struct MarkovCollection {
    // pub animal: MarkovData,
    pub gem: MarkovData,
    pub magic: MarkovData,
    pub metal: MarkovData,
    pub monster: MarkovData,
    pub name: MarkovData,
    pub plant: MarkovData,
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
                    Some(&';') | None => break,
                    Some(&c) => c,
                },
                None => break,
            });
        }
        // println!("{result:?}");
        if 5 < result.len() && result.len() < 15 {
            Some(
                // capitalize first letter of each word
                result
                    .split(' ')
                    .map(|word| {
                        let mut chars = word.chars();
                        chars.next().map_or(String::new(), |first| {
                            first.to_uppercase().collect::<String>() + chars.as_str()
                        })
                    })
                    .collect::<Vec<String>>()
                    .join(" "),
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
                .map(std::convert::AsRef::as_ref)
                .collect::<Vec<&str>>(),
        ))
    }

    pub fn from_strings(strings: &[&str]) -> Self {
        {
            let mut counts: HashMap<((char, char), char), u32> = HashMap::new();
            let mut starts = Vec::new();
            for &string_mixedcase in strings {
                let string = string_mixedcase.to_lowercase();
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
                        counts.get(&char_triple).map_or(1, |c| c + 1)
                    });
                }
            }
            let mut intermediate_counts: HashMap<(char, char), (Vec<char>, Vec<u32>)> =
                HashMap::new();
            for (&(k, character), &amount) in &counts {
                intermediate_counts.insert(k, {
                    let mut vectors = intermediate_counts
                        .get(&k)
                        .map_or((Vec::new(), Vec::new()), Clone::clone);
                    vectors.0.push(character);
                    vectors.1.push(amount);
                    vectors
                });
            }
            Self {
                starts,
                map: intermediate_counts
                    .iter()
                    .filter_map(|(&k, (chars, weights))| {
                        WeightedIndex::new(weights)
                            .map_or(None, |res| Some((k, (chars.clone(), weights.clone(), res))))
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
            let weight_scale = *weights.iter().max().unwrap() / 8 + 1;
            for character_index in 0..characters.len() {
                assert!(character_index < characters.len());
                bytes.push(
                    char_to_byte((
                        characters[character_index],
                        (weights[character_index] / weight_scale) as u8,
                    ))
                    .unwrap(),
                );
            }
            bytes.push(0);
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
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
                None | Some(0) => break,
                Some(&first_byte) => (
                    byte_to_char(first_byte).unwrap().0,
                    byte_to_char(*bytes_iter.next().unwrap()).unwrap().0,
                ),
            };
            let mut weights: Vec<(char, u32)> = Vec::new();
            // over each possible character that could come next
            loop {
                match bytes_iter.next() {
                    None | Some(0) => break,
                    Some(&val) => {
                        weights.push(byte_to_char(val).map(|(c, u)| (c, u32::from(u))).unwrap());
                    }
                }
            }
            intermediate_counts.insert(char_pair, weights.iter().copied().unzip());
        }
        Some(Self {
            starts,
            map: intermediate_counts
                .iter()
                .filter_map(|(&k, (chars, weights))| {
                    WeightedIndex::new(weights)
                        .map_or(None, |res| Some((k, (chars.clone(), weights.clone(), res))))
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
        char_part = char as u8 - 96;
    } else {
        println!("{} as u8 = {}", char, char as u8);
        return None;
    }
    Some(((weight.clamp(1, 8) - 1) << 5) + char_part)
}

fn byte_to_char(byte: u8) -> Option<(char, u8)> {
    use std::cmp::Ordering;

    match 27.cmp(&(byte & 0b0001_1111)) {
        Ordering::Less => None,
        Ordering::Equal => Some((';', (byte >> 5) + 1)),
        Ordering::Greater => match byte & 0b0001_1111 {
            0 => None,
            letter => Some(((letter + 96) as char, (byte >> 5) + 1)),
        },
    }
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use super::MarkovData;

    #[test]
    fn byte_to_char() {
        use super::byte_to_char;
        assert_eq!(byte_to_char(0b11100001), Some(('a', 8)));
        assert_eq!(byte_to_char(0b00000001), Some(('a', 1)));
        assert_eq!(byte_to_char(0b00011010), Some(('z', 1)));
        assert_eq!(byte_to_char(0b01111011), Some((';', 4)));
        assert_eq!(byte_to_char(0b01111100), None);
        assert_eq!(byte_to_char(0b00000000), None);
    }

    #[test]
    fn char_to_byte() {
        use super::char_to_byte;
        assert_eq!(char_to_byte(('a', 8)), Some(0b11100001));
        assert_eq!(char_to_byte(('z', 1)), Some(0b00011010));
        assert_eq!(char_to_byte((';', 4)), Some(0b01111011));
        assert_eq!(char_to_byte((' ', 4)), None);
        assert_eq!(char_to_byte(('`', 8)), None);
    }

    #[test]
    fn inverse_function() {
        use super::{byte_to_char, char_to_byte};
        assert_eq!(
            char_to_byte(byte_to_char(0b11100001).unwrap()),
            Some(0b11100001)
        );
        assert_eq!(
            byte_to_char(char_to_byte(('a', 4)).unwrap()),
            Some(('a', 4))
        );
        assert_eq!(
            char_to_byte(byte_to_char(0b11111011).unwrap()),
            Some(0b11111011)
        );
        assert_eq!(
            byte_to_char(char_to_byte((';', 4)).unwrap()),
            Some((';', 4))
        );
        assert_eq!(
            byte_to_char(char_to_byte(('z', 40)).unwrap()),
            Some(('z', 8))
        );
    }

    #[test]
    fn save_and_load() {
        let mut rng = thread_rng();
        let string_pool = ["strings"];

        let initialized_markov = MarkovData::from_strings(&string_pool);
        let bytes = initialized_markov.to_bytes();
        let loaded_markov = MarkovData::from_bytes(&bytes).unwrap();
        assert_eq!(String::from("Strings"), loaded_markov.sample(&mut rng))
    }

    #[test]
    fn capitalization_invariant() {
        let mut rng = thread_rng();
        let capital_pool = ["STRINGS"];
        let lowercase_pool = ["strings"];
        let capital_markov = MarkovData::from_strings(&capital_pool);
        let lowercase_markov = MarkovData::from_strings(&lowercase_pool);
        assert_eq!(
            capital_markov.sample(&mut rng),
            lowercase_markov.sample(&mut rng)
        );
    }
}
