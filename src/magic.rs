use rand::{rngs::ThreadRng, seq::IteratorRandom, Rng};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

use crate::{mkv::MarkovData, ItemType};

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

impl Ability {
    fn gen(rng: &mut ThreadRng) -> Self {
        let ability_type = AbilityType::iter().choose(rng).unwrap();
        match ability_type {
            AbilityType::Combat => Self {
                ability_type,
                strength: rng.gen_range(2..6),
                min_level: 2,
            },
            AbilityType::Homunculus => Self {
                ability_type,
                strength: rng.gen_range(2..5) * 10,
                min_level: rng.gen_range(2..6),
            },
            AbilityType::Portal => Self {
                ability_type,
                strength: rng.gen_range(2..5) * 10,
                min_level: rng.gen_range(6..12),
            },
            AbilityType::Youth => Self {
                ability_type,
                strength: rng.gen_range(2..6),
                min_level: 2,
            },
        }
    }
}

pub struct MagicSystem {
    pub material: ItemType,
    pub material_type: MaterialType,
    pub name: String,
    pub abilities: Vec<Ability>,
}

impl MagicSystem {
    pub fn gen(
        rng: &mut ThreadRng,
        markov_magic: &MarkovData,
        markov_gem: &MarkovData,
        markov_metal: &MarkovData,
        markov_plant: &MarkovData,
    ) -> Self {
        let material_type = MaterialType::iter().choose(rng).unwrap();
        let material_rarity = rng.gen_range(6..10);
        Self {
            material_type,
            material: ItemType {
                name: match material_type {
                    MaterialType::Plant => markov_plant,
                    MaterialType::Gem => markov_gem,
                    MaterialType::Metal => markov_metal,
                }
                .sample(rng),
                rarity: material_rarity,
                abundance: 2,
                value: 10,
                taming: 0,
            },
            name: markov_magic.sample(rng),
            abilities: (0..3).map(|_| Ability::gen(rng)).collect(),
        }
    }
}
