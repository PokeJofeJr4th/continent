#[allow(clippy::wildcard_imports)]
use crate::{
    magic::{Ability, AbilityType, MagicSystem, MaterialType},
    *,
};

fn json_array_to_usize(arr: &JsonValue, config: &Config) -> Option<usize> {
    match arr {
        JsonValue::Array(coords) => {
            let x = json_int(coords.get(0)?)? as usize;
            let y = json_int(coords.get(1)?)? as usize;
            Some(x + y * config.world_size.0)
        }
        _ => None,
    }
}

fn json_string(jsonvalue: &JsonValue) -> Option<String> {
    match jsonvalue {
        JsonValue::String(str) => Some(str.clone()),
        JsonValue::Short(str) => Some(String::from(*str)),
        _ => None,
    }
}

fn json_int(jsonvalue: &JsonValue) -> Option<i32> {
    match jsonvalue {
        JsonValue::Number(num) => Some(num.as_fixed_point_i64(0).unwrap_or_default() as i32),
        _ => None,
    }
}

fn json_float(jsonvalue: &JsonValue, depth: u16) -> Option<f32> {
    match jsonvalue {
        JsonValue::Number(num) => Some(
            num.as_fixed_point_i64(depth).unwrap_or_default() as f32 / 10.0f32.powf(depth as f32),
        ),
        _ => None,
    }
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

impl Jsonizable for Inventory {
    fn jsonize(&self, _config: &Config, items: &Items) -> JsonValue {
        JsonValue::from(
            self.iter()
                .enumerate()
                .filter_map(|(index, &amount)| {
                    if amount == 0.0 {
                        None
                    } else {
                        Some((items.all[index].to_string(items), amount))
                    }
                })
                .collect::<HashMap<String, f32>>(),
        )
    }

    fn dejsonize(src: &JsonValue, _config: &Config, items: &Items) -> Option<Self> {
        let JsonValue::Object(object) = src else {
            return None;
        };
        let map = items
            .all
            .iter()
            .map(|item| {
                object.get(&item.to_string(items)).map_or(0.0, |jsonvalue| {
                    json_float(jsonvalue, 2).unwrap_or_default()
                })
            })
            .collect();
        Some(Self(map))
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
            ($key: expr) => {
                match object.get("Plants") {
                    Some(JsonValue::Object(obj)) => obj
                        .iter()
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
                        .collect(),
                    _ => return None,
                }
            };
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
            let [rarity, abundance, value] = numerical_values[..] else { return None };
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
        match src {
            JsonValue::Object(object) => Some(Self {
                id: 0,
                tiles: match object.get("tiles") {
                    Some(JsonValue::Array(array)) => {
                        let mut tiles = Vec::new();
                        for tile in array {
                            tiles.push(json_array_to_usize(tile, config)?);
                        }
                        tiles
                    }
                    _ => return None,
                },
                resources: Inventory::dejsonize(object.get("resources")?, config, items)?,
                terrain: Terrain::dejsonize(object.get("terrain")?, config, items)?,
                adjacent_regions: match object.get("adjacent_regions") {
                    Some(JsonValue::Array(array)) => {
                        let mut regions = Vec::new();
                        for item in array {
                            regions.push(json_int(item)? as usize);
                        }
                        Some(regions)
                    }
                    _ => None,
                }?,
                monster: Monster::dejsonize(object.get("monster")?, config, items),
            }),
            _ => None,
        }
    }
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
        match src {
            JsonValue::Object(object) => Some(Self {
                alive: match object.get("alive") {
                    Some(JsonValue::Boolean(alive)) => *alive,
                    _ => return None,
                },
                location: json_array_to_usize(object.get("location")?, config)?,
                inventory: Inventory::dejsonize(object.get("inventory")?, config, items)?,
                species: json_string(object.get("species")?)?,
                name: json_string(object.get("name")?)?,
                desc: json_string(object.get("desc")?)?,
            }),
            _ => None,
        }
    }
}

impl Jsonizable for Snapshot {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        object! {
            population: self.population,
            production: self.production.jsonize(config, items),
            imports: self.imports.jsonize(config, items)
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        match src {
            JsonValue::Object(object) => Some(Self {
                population: object.get("population")?.as_fixed_point_i64(0).unwrap_or(0) as i32,
                production: Inventory::dejsonize(object.get("production")?, config, items)?,
                imports: Inventory::dejsonize(object.get("imports")?, config, items)?,
            }),
            _ => None,
        }
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
        match src {
            JsonValue::Object(object) => Some(Self {
                name: json_string(object.get("name")?)?,
                title: json_string(object.get("title")?)?,
                pos: json_array_to_usize(object.get("pos")?, config)?,
                origin: json_array_to_usize(object.get("origin")?, config)?,
                birth: json_int(object.get("birth")?)? as u32,
                age: json_int(object.get("age")?)? as u32,
                alive: match object.get("alive") {
                    Some(JsonValue::Boolean(bool)) => *bool,
                    _ => return None,
                },
                skills: match object.get("skills") {
                    Some(JsonValue::Object(object)) => {
                        let mut skills = HashMap::new();
                        for skill in Skill::iter() {
                            skills.insert(
                                skill,
                                json_int(object.get(skill.as_ref()).unwrap_or(&JsonValue::Null))
                                    .unwrap_or_default() as u8,
                            );
                        }
                        skills
                    }
                    _ => return None,
                },
                life: Vec::<HistoricalEvent>::dejsonize(object.get("life")?, config, items)?,
            }),
            _ => None,
        }
    }
}

impl SuperJsonizable for HistoricalEvent {
    fn s_jsonize(&self) -> JsonValue {
        object! {
            Type: "Event",
            Time: self.time,
            Desc: self.description.clone()
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        match src {
            JsonValue::Object(object) => Some(Self {
                time: json_int(object.get("Time")?)? as u32,
                description: json_string(object.get("Desc")?)?,
            }),
            _ => None,
        }
    }
}

impl Jsonizable for City {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        object! {
            pos: usize_to_vec(self.pos, config),
            name: self.name.clone(),
            population: self.population,
            homunculi: self.homunculi,
            NPCs: self.npcs.jsonize(config, items),
            data: self.data.jsonize(config, items),
            imports: self.imports.jsonize(config, items),
            production: self.production.jsonize(config, items),
            resources: self.resources.jsonize(config, items),
            economy: self.economy.jsonize(config, items),
            resource_gathering: self.resource_gathering.jsonize(config, items),
            history: array![],
            trade: array![],
            artifacts: array![],
            cultural_values: object!{},
            library: object!{}
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        // println!("dj city");
        let JsonValue::Object(object) = src else {
            return None;
        };
        Some(Self {
            name: json_string(object.get("name")?)?,
            pos: json_array_to_usize(object.get("pos")?, config)?,
            npcs: Vec::<Npc>::dejsonize(object.get("NPCs")?, config, items)?,
            population: json_int(object.get("population")?)?,
            homunculi: json_int(object.get("homunculi")?)?,
            resources: Inventory::dejsonize(object.get("resources")?, config, items)?,
            economy: Inventory::dejsonize(object.get("economy")?, config, items)?,
            resource_gathering: Inventory::dejsonize(
                object.get("resource_gathering")?,
                config,
                items,
            )?,
            data: HashMap::<String, Snapshot>::dejsonize(object.get("data")?, config, items)?,
            production: Inventory::dejsonize(object.get("production")?, config, items)?,
            imports: Inventory::dejsonize(object.get("imports")?, config, items)?,
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
        match src {
            JsonValue::Object(object) => Some(Self {
                gen_radius: json_int(object.get("GEN_RADIUS")?)? as usize,
                world_size: match object.get("WORLD_SIZE") {
                    Some(JsonValue::Array(array)) => {
                        let x = json_int(array.get(0)?)? as usize;
                        let y = json_int(array.get(1)?)? as usize;
                        (x, y)
                    }
                    _ => return None,
                },
                coastal_city_density: json_float(object.get("COASTAL_CITY_DENSITY")?, 4)?,
                inland_city_density: json_float(object.get("INLAND_CITY_DENSITY")?, 4)?,
                production_constant: json_float(object.get("PRODUCTION_CONSTANT")?, 6)?,
                population_constant: json_float(object.get("POPULATION_CONSTANT")?, 6)?,
                mineral_depletion: json_float(object.get("MINERAL_DEPLETION")?, 6)?,
                notable_npc_threshold: json_int(object.get("NOTABLE_NPC_THRESHOLD")?)? as u8,
                trade_volume: json_float(object.get("TRADE_VOLUME")?, 3)?,
                trade_quantity: json_int(object.get("TRADE_QUANTITY")?)?,
            }),
            _ => None,
        }
    }
}

impl SuperJsonizable for Species {
    fn s_jsonize(&self) -> JsonValue {
        JsonValue::from(self.as_ref())
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        json_string(src).and_then(|str| match str.as_ref() {
            "Leviathan" => Some(Self::Leviathan),
            "Dragon" => Some(Self::Dragon),
            "Beast" => Some(Self::Beast),
            "Worm" => Some(Self::Worm),
            _ => None,
        })
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
        match src {
            JsonValue::Object(object) => Some(Self {
                ability_type: match json_string(object.get("Type")?)?.as_ref() {
                    "Combat" => AbilityType::Combat,
                    "Homunculus" => AbilityType::Homunculus,
                    "Portal" => AbilityType::Portal,
                    "Youth" => AbilityType::Youth,
                    _ => return None,
                },
                strength: json_int(object.get("Strength")?)? as u8,
                min_level: json_int(object.get("Min Level")?)? as u8,
            }),
            _ => None,
        }
    }
}

impl Jsonizable for MagicSystem {
    fn jsonize(&self, config: &Config, items: &Items) -> JsonValue {
        object! {
            Material: [self.material.name.clone(), [self.material.rarity, self.material.abundance, self.material.value], self.material_type.as_ref()],
            Localization: "Ubiquitous",
            Name: self.name.clone(),
            Abilities: self.abilities.jsonize(config, items)
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config, items: &Items) -> Option<Self> {
        // println!("dj magic");
        let JsonValue::Object(object) = src else {
            return None;
        };
        let Some(JsonValue::Array(arr)) = object.get("Material") else {
                    return None
                };
        let Some(JsonValue::Array(numbers)) = arr.get(1) else {
                    return None
                };
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
        let JsonValue::Object(object) = src else {
            return None
        };
        let config = Config::s_dejsonize(object.get("Config")?)?;
        let items = Items::s_dejsonize(object.get("Items")?)?;
        let region_list = {
            match object.get("RegionList") {
                Some(JsonValue::Array(arr)) => {
                    let mut region_list = Vec::new();
                    for (id, region) in arr.iter().enumerate() {
                        let mut region = Region::dejsonize(region, &config, &items)?;
                        region.id = id;
                        region_list.push(region);
                    }
                    Some(region_list)
                }
                _ => None,
            }
        }?;
        let trade_connections = {
            let Some(JsonValue::Object(tcons)) = object.get("trade_connections") else {return None};
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
            for &tile in &region.tiles {
                region_map[tile] = region.id;
            }
        }
        Some(Self {
            config,
            magic: MagicSystem::dejsonize(object.get("Magic")?, &config, &items)?,
            current_year: json_int(object.get("current_year")?)? as u32,
            region_list,
            city_list: Vec::<City>::dejsonize(object.get("CityList")?, &config, &items)?
                .iter()
                .map(|c| (c.pos, c.clone()))
                .collect(),
            trade_connections_list: trade_connections.keys().copied().collect(),
            trade_connections,
            items,
            region_map,
        })
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
            config: self.config.s_jsonize(),
            items: self.items_src.clone()
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        let JsonValue::Object(object) = src else {
            return None
        };
        let JsonValue::Array(items_src) = object.get("items")? else {
            return None
        };
        let items_src: Vec<String> = items_src.iter().filter_map(json_string).collect();
        None
    }
}
