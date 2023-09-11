use json::{object, JsonValue};
use rand::{prelude::Distribution, rngs::ThreadRng, seq::SliceRandom, Rng};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

use crate::{
    get_adj,
    jsonize::{json_array_to_usize, json_int, json_string, Jsonizable, SuperJsonizable},
    mkv::MarkovData,
    sim::Item,
    usize_to_vec, Config, Items,
};

use super::Inventory;

#[derive(Debug, Clone, Copy, AsRefStr, PartialEq, Eq, EnumIter)]
pub enum Terrain {
    Ocean,
    Plain,
    Forest,
    Mountain,
    Desert,
    Jungle,
}

impl Terrain {
    pub fn monster_types(self) -> Vec<Species> {
        match self {
            Self::Ocean => vec![Species::Leviathan],
            Self::Plain => vec![Species::Dragon, Species::Beast],
            Self::Forest => vec![Species::Beast],
            Self::Mountain => vec![Species::Dragon],
            Self::Desert => vec![Species::Worm, Species::Dragon],
            Self::Jungle => vec![Species::Beast, Species::Worm],
        }
    }

    pub fn color(&self) -> Vec<u8> {
        match &self {
            Self::Ocean => vec![70, 90, 140],
            Self::Plain => vec![120, 140, 80],
            Self::Forest => vec![90, 150, 80],
            Self::Mountain => vec![96, 96, 96],
            Self::Desert => vec![160, 140, 90],
            Self::Jungle => vec![40, 130, 80],
        }
    }
}

impl Jsonizable for Terrain {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        object! {
            Resources: {
                Animal: 0.3, Fish: 0.0, Plant: 0.9, Metal: 0.1, Gemstone: 0.1
            },
            Monsters: self.monster_types().jsonize(config, items),
            Color: self.color()
        }
    }

    fn dejsonize(src: &JsonValue, _config: &Config, _items: &Items) -> Option<Self> {
        match json_string(src)?.as_ref() {
            "Ocean" => Some(Self::Ocean),
            "Plain" => Some(Self::Plain),
            "Forest" => Some(Self::Forest),
            "Mountain" => Some(Self::Mountain),
            "Desert" => Some(Self::Desert),
            "Jungle" => Some(Self::Jungle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, AsRefStr, PartialEq, Eq, EnumIter)]
pub enum Species {
    Leviathan,
    Dragon,
    Beast,
    Worm,
}

impl SuperJsonizable for Species {
    fn s_jsonize(&self) -> JsonValue {
        JsonValue::from(self.as_ref())
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        match json_string(src)?.as_ref() {
            "Leviathan" => Some(Self::Leviathan),
            "Dragon" => Some(Self::Dragon),
            "Beast" => Some(Self::Beast),
            "Worm" => Some(Self::Worm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Region {
    id: usize,
    tiles: Vec<usize>,
    resources: Inventory,
    terrain: Terrain,
    adjacent_regions: Vec<usize>,
    monster: Option<Monster>,
}

impl Region {
    pub const fn id(&self) -> usize {
        self.id
    }

    pub fn set_id(&mut self, id: usize) {
        self.id = id
    }

    pub const fn tiles(&self) -> &Vec<usize> {
        &self.tiles
    }

    pub const fn resources(&self) -> &Inventory {
        &self.resources
    }

    pub const fn terrain(&self) -> Terrain {
        self.terrain
    }

    pub fn set_terrain(&mut self, terrain: Terrain) {
        self.terrain = terrain;
    }
}

impl Jsonizable for Region {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        object! {
            tiles: self.tiles.iter().map(|&tile| usize_to_vec(tile, config)).collect::<Vec<Vec<usize>>>(),
            resources: self.resources.jsonize(config, items),
            terrain: self.terrain.as_ref(),
            adjacent_regions: self.adjacent_regions.clone(),
            ancestor_race: "Human",
            demographics: object!{Human: 1.0},
            monster: self.monster.clone().map(|m| m.jsonize(config, items))
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        // println!("dj region");
        let JsonValue::Object(object) = src else { return None };
        let Some(JsonValue::Array(tiles_array)) = object.get("tiles") else { return None };
        let Some(JsonValue::Array(adj_array)) = object.get("adjacent_regions") else { return None };
        Some(Self {
            id: 0,
            tiles: {
                let mut tiles = Vec::new();
                for tile in tiles_array {
                    tiles.push(json_array_to_usize(tile, config)?);
                }
                tiles
            },
            resources: Inventory::dejsonize(object.get("resources")?, config, items)?,
            terrain: Terrain::dejsonize(object.get("terrain")?, config, items)?,
            adjacent_regions: {
                let mut regions = Vec::new();
                for item in adj_array {
                    regions.push(json_int(item)? as usize);
                }
                regions
            },
            monster: Monster::dejsonize(object.get("monster")?, config, items),
        })
    }
}

impl Region {
    pub fn gen(
        id: usize,
        region_map: &[usize],
        rng: &mut ThreadRng,
        region_count: usize,
        markov_data_monster: &MarkovData,
        config: &Config,
        items: &Items,
    ) -> Self {
        let tiles: Vec<usize> = (0..(config.world_size.0 * config.world_size.1))
            .filter(|&i| region_map[i] == id)
            .collect();
        let terrain = {
            let ter_iter = Terrain::iter().collect::<Vec<_>>();
            let ter = ter_iter.choose(rng);
            match ter {
                Some(&terrain) => terrain,
                None => Terrain::Ocean,
            }
        };
        let resources = {
            let (metal, gem, plant, animal) = match terrain {
                Terrain::Plain => (0.2, 0.1, 0.4, 0.9),
                Terrain::Forest => (0.1, 0.2, 0.9, 0.4),
                Terrain::Mountain => (0.9, 0.4, 0.2, 0.1),
                Terrain::Desert => (0.4, 0.9, 0.1, 0.2),
                Terrain::Jungle => (0.1, 0.4, 0.9, 0.2),
                Terrain::Ocean => (0.0, 0.0, 0.0, 0.0),
            };

            let mut resources = Inventory::default(items);

            macro_rules! run_type {
                ($resource : expr, $resource_item : expr, $resource_names : expr) => {
                    for resource_type in 0..$resource_names.len() {
                        if rng.gen::<f32>() < $resource {
                            resources.set(
                                $resource_item(resource_type as u8).to_index(items).unwrap(),
                                rng.gen::<f32>().mul_add($resource, 1.0),
                            );
                        }
                    }
                };
            }
            run_type!(metal, Item::Metal, items.metals);
            run_type!(gem, Item::Gem, items.gems);
            run_type!(plant, Item::Plant, items.plants);
            run_type!(animal, Item::WildAnimal, items.animals);
            resources.set(0, rng.gen::<f32>() * 2.0);
            resources
        };
        Self {
            id,
            tiles: tiles.clone(),
            resources,
            terrain,
            adjacent_regions: (0..region_count)
                .filter(|&neighbor_region| {
                    tiles.iter().any(|&tile| {
                        get_adj(tile, 1, config)
                            .iter()
                            .any(|&local_region| local_region == neighbor_region)
                    })
                })
                .collect(),
            monster: Some(Monster::gen(
                rng,
                terrain,
                &tiles,
                items,
                markov_data_monster,
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Monster {
    alive: bool,
    location: usize,
    inventory: Inventory,
    species: String,
    name: String,
    desc: String,
}

impl Jsonizable for Monster {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        object! {
            name: self.name.clone(),
            species: self.species.clone(),
            desc: self.desc.clone(),
            inventory: self.inventory.jsonize(config, items),
            alive: self.alive,
            location: usize_to_vec(self.location, config)
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        let JsonValue::Object(object) = src else { return None };
        let Some(JsonValue::Boolean(alive)) = object.get("alive") else { return None };
        Some(Self {
            alive: *alive,
            location: json_array_to_usize(object.get("location")?, config)?,
            inventory: Inventory::dejsonize(object.get("inventory")?, config, items)?,
            species: json_string(object.get("species")?)?,
            name: json_string(object.get("name")?)?,
            desc: json_string(object.get("desc")?)?,
        })
    }
}

impl Monster {
    pub fn gen(
        rng: &mut ThreadRng,
        terrain: Terrain,
        tiles: &[usize],
        items: &Items,
        markov_data_monster: &MarkovData,
    ) -> Self {
        let species = *terrain.monster_types().choose(rng).unwrap();
        Self {
            alive: true,
            location: *tiles.choose(rng).unwrap(),
            inventory: Inventory::default(items),
            species: String::from(species.as_ref()),
            name: markov_data_monster.sample(rng),
            desc: {
                let color = ["red", "blue", "black", "white", "green", "gray"]
                    .choose(rng)
                    .unwrap();
                match species {
                    Species::Leviathan => format!(
                        "a giant sea creature with {} tentacles, a {}, and {}, {} skin",
                        (rng.gen_range(3..=8)) * 2,
                        ["chitinous beak", "toothy maw"].choose(rng).unwrap(),
                        ["slimy", "smooth", "rough", "bumpy"].choose(rng).unwrap(),
                        color
                    ),
                    Species::Dragon => format!(
                        "a great winged reptile with {} horns and claws and {} scales. It {}",
                        ["engraved", "long", "sharpened", "serrated"]
                            .choose(rng)
                            .unwrap(),
                        color,
                        [
                            "is adorned with arcane sigils",
                            "wears bone jewelry",
                            "has a prominent scar"
                        ]
                        .choose(rng)
                        .unwrap()
                    ),
                    Species::Beast => {
                        let (species1, species2) = {
                            let mut slice = [
                                "bear", "beaver", "gorilla", "coyote", "wolf", "bird", "deer",
                                "owl", "lizard", "moose", "spider", "insect", "lion",
                            ]
                            .choose_multiple(rng, 2);
                            (slice.next().unwrap(), slice.next().unwrap())
                        };
                        let (species, part): &(&str, &str) = [
                            ("bird", "wings"),
                            ("bat", "wings"),
                            ("snake", "fangs"),
                            ("deer", "antlers"),
                            ("moose", "antlers"),
                            ("spider", "legs"),
                            ("scorpion", "stinger"),
                            ("elephant", "tusks"),
                        ]
                        .choose(rng)
                        .unwrap();
                        format!(
                            "an oversized {} {} the {} of a {}",
                            species1,
                            {
                                if species1 == species2 {
                                    String::from("with")
                                } else {
                                    format!("with the head of a {species2} and")
                                }
                            },
                            part,
                            species
                        )
                    }
                    Species::Worm => format!(
                        "an enormous worm with {} plating {}",
                        color,
                        [
                            "engraved with symbols",
                            "covered in spikes",
                            "and a fleshy sail along its spine"
                        ]
                        .choose(rng)
                        .unwrap()
                    ),
                }
            },
        }
    }
}
