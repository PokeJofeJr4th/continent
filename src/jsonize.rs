use crate::sim::usize_to_vec;
#[allow(clippy::wildcard_imports)]
use crate::{
    magic::{Ability, AbilityType, MagicSystem, MaterialType},
    *,
};
use std::fs;

pub fn json_array_to_usize(arr: &JsonValue, config: &Config) -> Option<usize> {
    let JsonValue::Array(coords) = arr else { return None };
    let x = json_int(coords.get(0)?)? as usize;
    let y = json_int(coords.get(1)?)? as usize;
    Some(x + y * config.world_size.0)
}

pub fn json_string(jsonvalue: &JsonValue) -> Option<String> {
    match jsonvalue {
        JsonValue::String(str) => Some(str.clone()),
        JsonValue::Short(str) => Some(String::from(*str)),
        _ => None,
    }
}

pub fn json_int(jsonvalue: &JsonValue) -> Option<i32> {
    let JsonValue::Number(num) = jsonvalue else { return None };
    Some(num.as_fixed_point_i64(0).unwrap_or_default() as i32)
}

pub fn json_float(jsonvalue: &JsonValue, depth: u16) -> Option<f32> {
    let JsonValue::Number(num) = jsonvalue else { return None };
    Some(num.as_fixed_point_i64(depth).unwrap_or_default() as f32 / 10.0f32.powf(depth as f32))
}

pub trait Jsonizable: Sized {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue;
    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self>;
}

pub trait SuperJsonizable: Sized {
    fn s_jsonize(&self) -> JsonValue;
    fn s_dejsonize(src: &JsonValue) -> Option<Self>;
}

impl<T: SuperJsonizable> Jsonizable for T {
    fn jsonize(&self, _config: &Config, _items: &Items) -> JsonValue {
        self.s_jsonize()
    }

    fn dejsonize(src: &JsonValue, _config: &Config, _items: &Items) -> Option<Self> {
        Self::s_dejsonize(src)
    }
}

impl<T: Jsonizable> Jsonizable for Vec<T> {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        JsonValue::Array(self.iter().map(|i| i.jsonize(config, items)).collect())
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        let JsonValue::Array(array) = src else { return None };
        let mut result: Self = Self::new();
        for item in array {
            result.push(T::dejsonize(item, config, items)?);
        }
        Some(result)
    }
}

// impl<T: SuperJsonizable> SuperJsonizable for Vec<T> {
//     fn s_jsonize(&self) -> JsonValue {
//         JsonValue::Array(self.iter().map(|i| i.s_jsonize()).collect())
//     }

//     fn s_dejsonize(src: &JsonValue) -> Option<Self> {
//         match src {
//             JsonValue::Array(array) => {
//                 let mut result: Self = Vec::new();
//                 for src in array {
//                     result.push(T::s_dejsonize(src)?)
//                 }
//                 Some(result)
//             }
//             _ => None,
//         }
//     }
// }

impl<T: Jsonizable> Jsonizable for HashMap<String, T> {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        let mut object = json::object::Object::new();
        self.iter()
            .for_each(|(key, value)| object.insert(key, value.jsonize(config, items)));
        JsonValue::Object(object)
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        let JsonValue::Object(object) = src else {
            return None
        };
        let mut res = Self::new();
        for (key, value) in object.iter() {
            res.insert(String::from(key), T::dejsonize(value, config, items)?);
        }
        Some(res)
    }
}

impl SuperJsonizable for Items {
    fn s_jsonize(&self) -> JsonValue {
        object! {
            Animals: self.animals.iter()
                .map(|a| (a.name.clone(), vec![a.rarity, a.abundance, a.value, a.taming]))
                .collect::<HashMap<String, Vec<u8>>>(),
            Plants: self.plants.iter()
                .map(|a| (a.name.clone(), vec![a.rarity, a.abundance, a.value]))
                .collect::<HashMap<String, Vec<u8>>>(),
            Gems: self.gems.iter()
                .map(|a| (a.name.clone(), vec![a.rarity, a.abundance, a.value]))
                .collect::<HashMap<String, Vec<u8>>>(),
            Metals: self.metals.iter()
                .map(|a| (a.name.clone(), vec![a.rarity, a.abundance, a.value]))
                .collect::<HashMap<String, Vec<u8>>>(),
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        let JsonValue::Object(object) = src else { return None };
        macro_rules! item_type {
            ($key: expr) => {{
                let Some(JsonValue::Object(obj)) = object.get($key) else { return None };
                obj.iter()
                    .filter_map(|(name, values)| match values {
                        JsonValue::Array(arr) => Some(ItemType {
                            name: String::from(name),
                            rarity: json_int(arr.get(0)?)? as u8,
                            abundance: json_int(arr.get(1)?)? as u8,
                            value: json_int(arr.get(2)?)? as u8,
                            taming: arr
                                .get(3)
                                .map_or(0, |jsonvalue| json_int(jsonvalue).unwrap_or(0))
                                as u8,
                        }),
                        _ => None,
                    })
                    .collect()
            }};
        }

        Some(Self::from_item_types(
            item_type!("Plants"),
            item_type!("Metals"),
            item_type!("Gems"),
            item_type!("Animals"),
        ))
    }
}

impl Items {
    fn from_txt(txt: &str) -> Option<Self> {
        // println!("items from txt: {txt}");
        let lines = txt.split('\n').collect::<Vec<&str>>();
        let chunks = lines.chunks_exact(2);
        let mut plants = Vec::new();
        let mut metals = Vec::new();
        let mut gems = Vec::new();
        let mut animals = Vec::new();
        for chunk in chunks {
            let [t, name] = chunk[0].split(':').collect::<Vec<&str>>()[..] else {
                return None
            };
            let numerical_values = chunk[1]
                .split(',')
                .map(str::parse::<u8>)
                .filter_map(Result::ok)
                .collect::<Vec<u8>>();
            let &rarity = numerical_values.first()?;
            let &abundance = numerical_values.get(1)?;
            let &value = numerical_values.get(2)?;
            let &taming = numerical_values.get(3).unwrap_or(&0);
            match t {
                "animal" => &mut animals,
                "plant" => &mut plants,
                "metal" => &mut metals,
                "gem" => &mut gems,
                _ => return None,
            }
            .push(ItemType {
                name: String::from(name),
                rarity,
                abundance,
                value,
                taming,
            });
        }
        Some(Self::from_item_types(plants, metals, gems, animals))
    }
}

impl Jsonizable for Npc {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        object! {
            name: self.name.clone(),
            title: self.title.clone(),
            pos: usize_to_vec(self.pos, config),
            origin: usize_to_vec(self.origin, config),
            birth: self.birth,
            age: self.age,
            race: "Human",
            alive: self.alive,
            skills: object!{},
            inventory: object!{},
            life: self.life.jsonize(config, items),
            reputation: 10,
            skills: self.skills.clone(),
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        let JsonValue::Object(object) = src else { return None };
        let Some(JsonValue::Boolean(alive)) = object.get("alive") else { return None };
        let Some(JsonValue::Object(skills_obj)) = object.get("skills") else { return None };
        Some(Self {
            name: json_string(object.get("name")?)?,
            title: json_string(object.get("title")?)?,
            pos: json_array_to_usize(object.get("pos")?, config)?,
            origin: json_array_to_usize(object.get("origin")?, config)?,
            birth: json_int(object.get("birth")?)? as u32,
            age: json_int(object.get("age")?)? as u32,
            alive: *alive,
            skills: {
                let mut skills = HashMap::new();
                for skill in Skill::iter() {
                    skills.insert(
                        skill,
                        json_int(skills_obj.get(skill.as_ref()).unwrap_or(&JsonValue::Null))
                            .unwrap_or_default() as u8,
                    );
                }
                skills
            },
            life: Vec::<HistoricalEvent>::dejsonize(object.get("life")?, config, items)?,
        })
    }
}

impl SuperJsonizable for Config {
    fn s_jsonize(&self) -> JsonValue {
        object! {
            GEN_RADIUS: self.gen_radius,
            WORLD_SIZE: vec![self.world_size.0, self.world_size.1],
            COASTAL_CITY_DENSITY: self.coastal_city_density,
            INLAND_CITY_DENSITY: self.inland_city_density,
            PRODUCTION_CONSTANT: self.production_constant,
            POPULATION_CONSTANT: self.population_constant,
            NOTABLE_NPC_THRESHOLD: self.notable_npc_threshold,
            MINERAL_DEPLETION: self.mineral_depletion,
            TRADE_VOLUME: self.trade_volume,
            TRADE_QUANTITY: self.trade_quantity,
            ARMY_SIZE: 200,
            ARMY_PARAMETER: 0.7
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        // println!("dj config");
        let JsonValue::Object(object) = src else { return None };
        let Some(JsonValue::Array(world_size)) = object.get("WORLD_SIZE") else { return None };
        Some(Self {
            gen_radius: json_int(object.get("GEN_RADIUS")?)? as usize,
            world_size: {
                let x = json_int(world_size.get(0)?)? as usize;
                let y = json_int(world_size.get(1)?)? as usize;
                (x, y)
            },
            coastal_city_density: json_float(object.get("COASTAL_CITY_DENSITY")?, 4)?,
            inland_city_density: json_float(object.get("INLAND_CITY_DENSITY")?, 4)?,
            production_constant: json_float(object.get("PRODUCTION_CONSTANT")?, 6)?,
            population_constant: json_float(object.get("POPULATION_CONSTANT")?, 6)?,
            mineral_depletion: json_float(object.get("MINERAL_DEPLETION")?, 6)?,
            notable_npc_threshold: json_int(object.get("NOTABLE_NPC_THRESHOLD")?)? as u8,
            trade_volume: json_float(object.get("TRADE_VOLUME")?, 3)?,
            trade_quantity: json_int(object.get("TRADE_QUANTITY")?)?,
        })
    }
}

impl SuperJsonizable for Ability {
    fn s_jsonize(&self) -> JsonValue {
        object! {
            Type: self.ability_type.as_ref(),
            Strength: self.strength,
            Component: "Gem",
            "Min Level": self.min_level
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        // println!("dj ability");
        let JsonValue::Object(object) = src else { return None };
        Some(Self {
            ability_type: match json_string(object.get("Type")?)?.as_ref() {
                "Combat" => AbilityType::Combat,
                "Homunculus" => AbilityType::Homunculus,
                "Portal" => AbilityType::Portal,
                "Youth" => AbilityType::Youth,
                _ => return None,
            },
            strength: json_int(object.get("Strength")?)? as u8,
            min_level: json_int(object.get("Min Level")?)? as u8,
        })
    }
}

impl Jsonizable for MagicSystem {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        object! {
            Material: [self.material.name.as_str(), [self.material.rarity, self.material.abundance, self.material.value], self.material_type.as_ref()],
            Localization: "Ubiquitous",
            Name: self.name.clone(),
            Abilities: self.abilities.jsonize(config, items)
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        // println!("dj magic");
        let JsonValue::Object(object) = src else { return None; };
        let Some(JsonValue::Array(arr)) = object.get("Material") else { return None };
        let Some(JsonValue::Array(numbers)) = arr.get(1) else { return None };
        let material = ItemType {
            name: json_string(arr.get(0)?)?,
            rarity: json_int(numbers.get(0)?)? as u8,
            abundance: json_int(numbers.get(1)?)? as u8,
            value: json_int(numbers.get(2)?)? as u8,
            taming: 0,
        };
        Some(Self {
            material: material.clone(),
            material_type: match json_string(arr.get(2)?)?.as_ref() {
                "Gem" => MaterialType::Gem,
                "Metal" => MaterialType::Metal,
                "Plant" => MaterialType::Plant,
                _ => return None,
            },
            name: json_string(object.get("Name")?)?,
            abilities: Vec::<Ability>::dejsonize(object.get("Abilities")?, config, items)?,
            index: items.all.iter().position(|item| match item {
                Item::Plant(plant) => items.plants[*plant as usize].name == material.name,
                Item::Metal(metal) => items.metals[*metal as usize].name == material.name,
                Item::Gem(gem) => items.gems[*gem as usize].name == material.name,
                _ => false,
            }),
        })
    }
}

impl SuperJsonizable for World {
    fn s_jsonize(&self) -> JsonValue {
        json::object! {
            file_type: "save",
            RegionList: self.region_list.jsonize(&self.config, &self.items),
            CityList: self.city_list.values().cloned().collect::<Vec<City>>().jsonize(&self.config, &self.items),
            trade_connections: self.trade_connections.iter().map(|((first, second), &strength)| (format!("[{}, {}, {}, {}]", first % self.config.world_size.0, first / self.config.world_size.0, second % self.config.world_size.0, second / self.config.world_size.0), strength)).collect::<HashMap<String, i32>>(),
            Biomes: {
                Desert: Terrain::Desert.jsonize(&self.config, &self.items),
                Forest: Terrain::Forest.jsonize(&self.config, &self.items),
                Jungle: Terrain::Jungle.jsonize(&self.config, &self.items),
                Mountain: Terrain::Mountain.jsonize(&self.config, &self.items),
                Ocean: Terrain::Ocean.jsonize(&self.config, &self.items),
                Plain: Terrain::Plain.jsonize(&self.config, &self.items)},
            Items: self.items.s_jsonize(),
            Magic: self.magic.jsonize(&self.config, &self.items),
            current_year: self.current_year,
            Config: self.config.jsonize(&self.config, &self.items)
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        let JsonValue::Object(object) = src else { return None };
        let Some(JsonValue::Object(tcons)) = object.get("trade_connections") else {return None};
        let Some(JsonValue::Array(arr)) = object.get("RegionList") else { return None; };
        if &json_string(object.get("file_type")?)? != "save" {
            return None;
        };
        let config = Config::s_dejsonize(object.get("Config")?)?;
        let items = Items::s_dejsonize(object.get("Items")?)?;
        let region_list: Vec<Region> = arr
            .iter()
            .enumerate()
            .filter_map(|(id, region)| {
                Region::dejsonize(region, &config, &items).map(|mut region| {
                    region.set_id(id);
                    region
                })
            })
            .collect();
        let trade_connections = {
            let mut trade_connections = HashMap::new();
            for (k, v) in tcons.iter() {
                let key = {
                    let Ok(JsonValue::Array(arr)) = json::parse(k) else { return None };
                    (
                        json_array_to_usize(
                            &JsonValue::Array(vec![arr.get(0)?.clone(), arr.get(1)?.clone()]),
                            &config,
                        )?,
                        json_array_to_usize(
                            &JsonValue::Array(vec![arr.get(2)?.clone(), arr.get(3)?.clone()]),
                            &config,
                        )?,
                    )
                };
                trade_connections.insert(key, json_int(v)?);
            }
            trade_connections
        };
        let mut region_map = vec![0; config.world_size.0 * config.world_size.1];
        for region in &region_list {
            for &tile in region.tiles() {
                region_map[tile] = region.id();
            }
        }
        Some(Self {
            config,
            magic: MagicSystem::dejsonize(object.get("Magic")?, &config, &items)?,
            current_year: json_int(object.get("current_year")?)? as u32,
            region_list,
            city_list: Vec::<City>::dejsonize(object.get("CityList")?, &config, &items)?
                .iter()
                .map(|c| (c.pos(), c.clone()))
                .collect(),
            trade_connections_list: trade_connections.keys().copied().collect(),
            trade_connections,
            items,
            region_map,
        })
    }
}

impl World {
    pub fn from_file(
        src: &JsonValue,
        rng: &mut ThreadRng,
        markov: &MarkovCollection,
    ) -> Option<Self> {
        let JsonValue::Object(object) = src else {
            return None
        };
        match json_string(object.get("file_type")?)?.as_str() {
            "gen" => Some(WorldGen::s_dejsonize(src)?.sample(rng, markov)),
            "save" => Self::s_dejsonize(src),
            _ => None,
        }
    }
}

impl SuperJsonizable for WorldGen {
    fn s_jsonize(&self) -> JsonValue {
        json::object! {
            file_type: "gen",
            Biomes: {
                Desert: Terrain::Desert.jsonize(&self.config, &self.items),
                Forest: Terrain::Forest.jsonize(&self.config, &self.items),
                Jungle: Terrain::Jungle.jsonize(&self.config, &self.items),
                Mountain: Terrain::Mountain.jsonize(&self.config, &self.items),
                Ocean: Terrain::Ocean.jsonize(&self.config, &self.items),
                Plain: Terrain::Plain.jsonize(&self.config, &self.items)},
            Config: self.config.s_jsonize(),
            Items: self.items_src.clone()
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        // println!("dj worldgen");
        let JsonValue::Object(object) = src else { return None };
        let Some(JsonValue::Array(items_src)) = object.get("Items") else { return None };
        if &json_string(object.get("file_type")?)? != "gen" {
            return None;
        };
        let items_strings: Vec<String> = items_src.iter().filter_map(json_string).collect();
        let items_str: String = items_strings
            .iter()
            // read the corresponding file
            .map(|file| fs::read_to_string(format!("objects/{file}.txt")))
            // filter out failures
            .filter_map(Result::ok)
            // merge the contents together
            .fold(String::new(), |acc, item| item + "\n" + &acc)
            // get rid of pesky carriage returns
            .replace('\r', "");
        let items = Items::from_txt(&items_str)?;
        Some(Self {
            config: Config::s_dejsonize(object.get("Config")?)?,
            items,
            items_src: items_strings,
        })
    }
}
