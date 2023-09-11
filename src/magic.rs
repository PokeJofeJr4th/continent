use rand::{
    distributions::Standard, prelude::Distribution, rngs::ThreadRng, seq::IteratorRandom, Rng,
};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

use crate::{mkv::MarkovCollection, ItemType};

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy, AsRefStr)]
pub enum MaterialType {
    Plant,
    Gem,
    Metal,
}

#[derive(EnumIter, AsRefStr, Debug, PartialEq, Eq, Clone, Copy)]
pub enum AbilityType {
    Combat,
    Homunculus,
    Portal,
    Youth,
}

#[derive(Debug, Clone, Copy)]
pub struct Ability {
    pub ability_type: AbilityType,
    pub strength: u8,
    pub min_level: u8,
}

impl Distribution<Ability> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Ability {
        let ability_type = AbilityType::iter().choose(rng).unwrap();
        match ability_type {
            AbilityType::Combat => Ability {
                ability_type,
                strength: rng.gen_range(2..6),
                min_level: rng.gen_range(1..3),
            },
            AbilityType::Homunculus => Ability {
                ability_type,
                strength: rng.gen_range(2..5) * 10,
                min_level: rng.gen_range(2..6),
            },
            AbilityType::Portal => Ability {
                ability_type,
                strength: rng.gen_range(2..5) * 10,
                min_level: rng.gen_range(6..12),
            },
            AbilityType::Youth => Ability {
                ability_type,
                strength: rng.gen_range(2..6),
                min_level: rng.gen_range(3..8),
            },
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct MagicSystem {
    pub material: ItemType,
    pub material_type: MaterialType,
    pub name: String,
    pub abilities: Vec<Ability>,
    pub index: Option<usize>,
}

impl MagicSystem {
    pub fn gen(rng: &mut ThreadRng, markov: &MarkovCollection) -> Self {
        let material_type = MaterialType::iter().choose(rng).unwrap();
        let material_rarity = rng.gen_range(6..10);
        Self {
            index: None,
            material_type,
            material: ItemType {
                name: match material_type {
                    MaterialType::Plant => &markov.plant,
                    MaterialType::Gem => &markov.gem,
                    MaterialType::Metal => &markov.metal,
                }
                .sample(rng),
                rarity: material_rarity,
                abundance: 2,
                value: 10,
                taming: 0,
            },
            name: markov.magic.sample(rng),
            abilities: (0..3).map(|_| rng.gen()).collect(),
        }
    }
}
