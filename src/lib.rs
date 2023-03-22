use std::collections::HashMap;

use rand::distributions::WeightedIndex;

pub type MarkovData = (
    Vec<(char, char)>,
    HashMap<(char, char), (Vec<char>, Vec<u32>, WeightedIndex<u32>)>,
);

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

pub fn bytes_to_markov(bytes: Vec<u8>) -> Option<MarkovData> {
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
    Some((
        starts,
        intermediate_counts
            .iter()
            .filter_map(|(&k, (chars, weights))| match WeightedIndex::new(weights) {
                Ok(res) => Some((k, (chars.clone(), weights.clone(), res))),
                Err(_) => None,
            })
            .collect(),
    ))
}
