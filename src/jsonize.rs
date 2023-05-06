use crate::*;

fn json_array_to_usize(arr: &JsonValue, config: &Config) -> Option<usize> {
    match arr {
        JsonValue::Array(coords) => {
            let xcoord = json_int(coords.get(0)?)? as usize;
            let ycoord = json_int(coords.get(1)?)? as usize;
            Some(xcoord + ycoord * config.world_size.0)
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
        JsonValue::Number(num) => Some(num.as_fixed_point_i64(depth).unwrap_or_default() as f32 / 10.0f32.powf(depth as f32)),
        _ => None
    }
}

pub trait Jsonizable: Sized {
    fn jsonize(&self, config: &Config) -> JsonValue;
    fn dejsonize(src: &JsonValue, config: &Config) -> Option<Self>;
}

pub trait SuperJsonizable: Sized {
    fn s_jsonize(&self) -> JsonValue;
    fn s_dejsonize(src: &JsonValue) -> Option<Self>;
}

impl<T: SuperJsonizable> Jsonizable for T {
    fn jsonize(&self, _config: &Config) -> JsonValue {
        self.s_jsonize()
    }

    fn dejsonize(src: &JsonValue, _config: &Config) -> Option<Self> {
        Self::s_dejsonize(src)
    }
}

impl<T: Jsonizable> Jsonizable for Vec<T> {
    fn jsonize(&self, config: &Config) -> JsonValue {
        JsonValue::Array(self.iter().map(|i| i.jsonize(config)).collect())
    }

    fn dejsonize(src: &JsonValue, config: &Config) -> Option<Self> {
        match src {
            JsonValue::Array(array) => {
                let mut result: Self = Vec::new();
                for src in array {
                    result.push(T::dejsonize(src, config)?);
                }
                Some(result)
            }
            _ => None,
        }
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
    fn jsonize(&self, config: &Config) -> JsonValue {
        let mut object = json::object::Object::new();
        self.iter()
            .for_each(|(key, value)| object.insert(key, value.jsonize(config)));
        JsonValue::Object(object)
    }

    fn dejsonize(src: &JsonValue, config: &Config) -> Option<Self> {
        match src {
            JsonValue::Object(object) => {
                let mut res = Self::new();
                for (key, value) in object.iter() {
                    res.insert(String::from(key), T::dejsonize(value, config)?);
                }
                Some(res)
            }
            _ => None,
        }
    }
}

impl SuperJsonizable for Inventory {
    fn s_jsonize(&self) -> JsonValue {
        JsonValue::from(
            self.iter()
                .enumerate()
                .filter_map(|(index, &amount)| {
                    if amount == 0.0 {
                        None
                    } else {
                        Some((format!("{}", Item::from(index)), amount))
                    }
                })
                .collect::<HashMap<String, f32>>(),
        )
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        match src {
            JsonValue::Object(object) => {
                let map = unsafe {
                    ALL_ITEMS
                        .iter()
                        .map(|item| {
                            object.get(&format!("{item}")).map_or(0.0, |jsonvalue| {
                                json_float(jsonvalue, 2).unwrap_or_default()
                            })
                        })
                        .collect()
                };
                Some(Self(map))
            }
            _ => None,
        }
    }
}

impl Jsonizable for Region {
    fn jsonize(&self, config: &Config) -> JsonValue {
        object! {
            tiles: self.tiles.iter().map(|&tile| usize_to_vec(tile, config)).collect::<Vec<Vec<usize>>>(),
            resources: self.resources.s_jsonize(),
            terrain: self.terrain.as_ref(),
            adjacent_regions: self.adjacent_regions.clone(),
            ancestor_race: "Human",
            demographics: object!{Human: 1.0},
            monster: self.monster.clone().map(|m| m.jsonize(config))
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config) -> Option<Self> {
        println!("dj region");
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
                resources: Inventory::s_dejsonize(object.get("resources")?)?,
                terrain: Terrain::dejsonize(object.get("terrain")?, config)?,
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
                monster: Monster::dejsonize(object.get("monster")?, config),
            }),
            _ => None,
        }
    }
}

impl Jsonizable for Monster {
    fn jsonize(&self, config: &Config) -> JsonValue {
        object! {
            name: self.name.clone(),
            species: self.species.clone(),
            desc: self.desc.clone(),
            inventory: self.inventory.jsonize(config),
            alive: self.alive,
            location: usize_to_vec(self.location, config)
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config) -> Option<Self> {
        match src {
            JsonValue::Object(object) => Some(Self {
                alive: match object.get("alive") {
                    Some(JsonValue::Boolean(alive)) => *alive,
                    _ => return None,
                },
                location: json_array_to_usize(object.get("location")?, config)?,
                inventory: Inventory::s_dejsonize(object.get("inventory")?)?,
                species: json_string(object.get("species")?)?,
                name: json_string(object.get("name")?)?,
                desc: json_string(object.get("desc")?)?,
            }),
            _ => None,
        }
    }
}

impl SuperJsonizable for Snapshot {
    fn s_jsonize(&self) -> JsonValue {
        object! {
            population: self.population,
            production: self.production.s_jsonize(),
            imports: self.imports.s_jsonize()
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        match src {
            JsonValue::Object(object) => Some(Self {
                population: object.get("population")?.as_fixed_point_i64(0).unwrap_or(0) as i32,
                production: Inventory::s_dejsonize(object.get("production")?)?,
                imports: Inventory::s_dejsonize(object.get("imports")?)?,
            }),
            _ => None,
        }
    }
}

impl Jsonizable for Npc {
    fn jsonize(&self, config: &Config) -> JsonValue {
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
            life: self.life.jsonize(config),
            reputation: 10,
            skills: self.skills.clone(),
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config) -> Option<Self> {
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
                                json_int(
                                    object.get(skill.as_ref()).unwrap_or(&JsonValue::Null)
                                    
                                )
                                .unwrap_or_default() as u8,
                            );
                        }
                        skills
                    }
                    _ => return None,
                },
                life: Vec::<HistoricalEvent>::dejsonize(object.get("life")?, config)?,
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
    fn jsonize(&self, config: &Config) -> JsonValue {
        object! {
            pos: usize_to_vec(self.pos, config),
            name: self.name.clone(),
            population: self.population,
            homunculi: 0,
            NPCs: self.npcs.jsonize(config),
            data: self.data.jsonize(config),
            imports: self.imports.jsonize(config),
            production: self.production.jsonize(config),
            resources: self.resources.jsonize(config),
            economy: self.economy.jsonize(config),
            resource_gathering: self.resource_gathering.jsonize(config),
            history: array![],
            trade: array![],
            artifacts: array![],
            cultural_values: object!{},
            library: object!{}
        }
    }

    fn dejsonize(src: &JsonValue, config: &Config) -> Option<Self> {
        match src {
            JsonValue::Object(object) => Some(Self {
                name: json_string(object.get("name")?)?,
                pos: json_array_to_usize(object.get("pos")?, config)?,
                npcs: Vec::<Npc>::dejsonize(object.get("NPCs")?, config)?,
                population: json_int(object.get("population")?)?,
                resources: Inventory::s_dejsonize(object.get("resources")?)?,
                economy: Inventory::s_dejsonize(object.get("economy")?)?,
                resource_gathering: Inventory::s_dejsonize(object.get("resource_gathering")?)?,
                data: HashMap::<String, Snapshot>::dejsonize(object.get("data")?, config)?,
                production: Inventory::s_dejsonize(object.get("production")?)?,
                imports: Inventory::s_dejsonize(object.get("imports")?)?,
            }),
            _ => None,
        }
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
            TRADE_VOLUME: self.trade_volume,
            TRADE_QUANTITY: self.trade_quantity,
            ARMY_SIZE: 200,
            ARMY_PARAMETER: 0.7
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        match src {
            JsonValue::Object(object) => Some(Self {
                gen_radius: json_int(object.get("GEN_RADIUS")?)? as usize,
                world_size: match object.get("WORLD_SIZE") {
                    Some(JsonValue::Array(array)) => {
                        let xsize = json_int(array.get(0)?)? as usize;
                        let ysize = json_int(array.get(1)?)? as usize;
                        (xsize, ysize)
                    }
                    _ => return None,
                },
                coastal_city_density: json_float(object.get("COASTAL_CITY_DENSITY")?, 3)?,
                inland_city_density: json_float(object.get("INLAND_CITY_DENSITY")?, 3)?,
                production_constant: json_float(object.get("PRODUCTION_CONSTANT")?, 3)?,
                population_constant: json_float(object.get("POPULATION_CONSTANT")?, 3)?,
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
        match json_string(src) {
            Some(str) => match str.as_ref() {
                "Leviathan" => Some(Species::Leviathan),
                "Dragon" => Some(Species::Dragon),
                "Beast" => Some(Species::Beast),
                "Worm" => Some(Species::Worm),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Jsonizable for Terrain {
    fn jsonize(&self, config: &Config) -> JsonValue {
        object! {
            Resources: {
                Animal: 0.3, Fish: 0.0, Plant: 0.9, Metal: 0.1, Gemstone: 0.1
            },
            Monsters: self.monster_types().jsonize(config),
            Color: self.color()
        }
    }
    fn dejsonize(src: &JsonValue, _config: &Config) -> Option<Self> {
        match json_string(src)?.as_ref() {
            "Ocean" => Some(Terrain::Ocean),
            "Plain" => Some(Terrain::Plain),
            "Forest" => Some(Terrain::Forest),
            "Mountain" => Some(Terrain::Mountain),
            "Desert" => Some(Terrain::Desert),
            "Jungle" => Some(Terrain::Jungle),
            _ => None,
        }
    }
}

impl SuperJsonizable for World {
    fn s_jsonize(&self) -> JsonValue {
        macro_rules! item_type {
            ($type: ident) => {
                $type::iter()
                    .map(|a| (String::from(a.as_ref()), vec![1, 1, 1, 1]))
                    .collect::<HashMap<String, Vec<i32>>>()
            };
        }

        json::object! {
            file_type: "save",
            RegionList: self.region_list.jsonize(&self.config),
            CityList: self.city_list.values().cloned().collect::<Vec<City>>().jsonize(&self.config),
            trade_connections: self.trade_connections.iter().map(|((first, second), &strength)| (format!("[{}, {}, {}, {}]", first % self.config.world_size.0, first / self.config.world_size.0, second % self.config.world_size.0, second / self.config.world_size.0), strength)).collect::<HashMap<String, i32>>(),
            Biomes: {
                Desert: Terrain::Desert.jsonize(&self.config),
                Forest: Terrain::Forest.jsonize(&self.config),
                Jungle: Terrain::Jungle.jsonize(&self.config),
                Mountain: Terrain::Mountain.jsonize(&self.config),
                Ocean: Terrain::Ocean.jsonize(&self.config),
                Plain: Terrain::Plain.jsonize(&self.config)},
            Items: {
                Animals: item_type!(Animal),
                Plants: item_type!(Plant),
                Gems: item_type!(Gem),
                Metals: item_type!(Metal)
            },
            Magic: {
                Material: ["Osmin", [1, 2, 9], "Metal"],
                Localization: "Ubiquitous",
                Name: "Supen",
                Abilities: [
                    {Type: "Combat", Component: "Gem", Strength: 2, "Min Level": 2}
                    ]
            },
            current_year: self.current_year,
            Config: self.config.jsonize(&self.config)
        }
    }

    fn s_dejsonize(src: &JsonValue) -> Option<Self> {
        match src {
            JsonValue::Object(object) => {
                let config = Config::s_dejsonize(object.get("Config")?)?;
                let region_list = {
                    match object.get("RegionList") {
                        Some(JsonValue::Array(arr)) => {
                            let mut region_list = Vec::new();
                            for (id, region) in arr.iter().enumerate() {
                                let mut region = Region::dejsonize(region, &config)?;
                                region.id = id;
                                region_list.push(region);
                            }
                            Some(region_list)
                        }
                        _ => None,
                    }
                }?;
                let trade_connections = match object.get("trade_connections") {
                    Some(JsonValue::Object(tcons)) => {
                        let mut trade_connections = HashMap::new();
                        for (k, v) in tcons.iter() {
                            let key = match json::parse(k) {
                                Ok(JsonValue::Array(arr)) => (
                                    json_array_to_usize(
                                        &JsonValue::Array(vec![
                                            arr.get(0)?.clone(),
                                            arr.get(1)?.clone(),
                                        ]),
                                        &config,
                                    )?,
                                    json_array_to_usize(
                                        &JsonValue::Array(vec![
                                            arr.get(2)?.clone(),
                                            arr.get(3)?.clone(),
                                        ]),
                                        &config,
                                    )?,
                                ),
                                _ => return None,
                            };
                            trade_connections.insert(key, json_int(v)?);
                        }
                        trade_connections
                    }
                    _ => return None,
                };
                println!("done trade");
                let mut region_map = vec![0; config.world_size.0 * config.world_size.1];
                for region in &region_list {
                    for &tile in &region.tiles {
                        region_map[tile] = region.id;
                    }
                }
                println!("done region map");
                Some(Self {
                    config,
                    current_year: json_int(object.get("current_year")?)? as u32,
                    region_list,
                    city_list: Vec::<City>::dejsonize(object.get("CityList")?, &config)?
                        .iter()
                        .map(|c| (c.pos, c.clone()))
                        .collect(),
                    trade_connections_list: trade_connections.keys().copied().collect(),
                    trade_connections,
                    region_map,
                })
            }
            _ => None,
        }
    }
}
