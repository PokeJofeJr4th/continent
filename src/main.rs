use json::*;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;
use std::cmp::min;
use std::collections::HashMap;
use std::{fmt, fs};
use strum::*;

macro_rules! get_adj {
    ($center: expr, $radius: expr) => {{
        if $radius == 0 {
            vec![
                $center + 1,
                $center - 1,
                $center + CONFIG.world_size.0,
                $center - CONFIG.world_size.0,
            ]
        } else {
            let mut adj: Vec<usize> = Vec::new();
            for x in 0..(2 * $radius + 1) {
                for y in 0..(2 * $radius + 1) {
                    if (x, y) == ($radius, $radius) {
                        continue;
                    }
                    let positive = $center + x + y * CONFIG.world_size.0;
                    let negative = $radius * (1 + CONFIG.world_size.0);
                    if negative > positive {
                        continue;
                    }
                    let adj_index: usize = positive - negative;
                    if adj_index < CONFIG.world_size.0 * CONFIG.world_size.1
                        && (adj_index / CONFIG.world_size.0
                            == ($center / CONFIG.world_size.0) + y - $radius)
                    {
                        adj.push(adj_index);
                    }
                }
            }
            // println!("{:?} => {adj:?}", $center);
            adj
        }
    }};
}

macro_rules! distance {
    ($a: expr, $b: expr) => {
        (((($a / CONFIG.world_size.0) as i32 - ($b / CONFIG.world_size.0) as i32).pow(2)
            + (($a % CONFIG.world_size.0) as i32 - ($b % CONFIG.world_size.0) as i32).pow(2))
            as f32)
            .sqrt()
    };
}

macro_rules! inverse_add {
    ($a: expr, $b: expr) => {
        ($a * $b) / ($a + $b)
    };
}

macro_rules! usize_to_vec {
    ($index: expr) => {
        vec![$index % CONFIG.world_size.0, $index / CONFIG.world_size.0]
    };
}

macro_rules! terrain_char {
    ($terrain: expr) => {{
        match $terrain {
            Terrain::Ocean => '~',     // Ocean
            Terrain::Plain => '%',     // Plain
            Terrain::Forest => '♣',  // Forest
            Terrain::Mountain => 'ߍ', // Mountain
            Terrain::Desert => '#',    // Desert
            Terrain::Jungle => '♠',  // Jungle
        }
    }};
}

impl From<Terrain> for JsonValue {
    fn from(terrain: Terrain) -> JsonValue {
        JsonValue::from(terrain.as_ref())
    }
}

impl From<Inventory> for JsonValue {
    fn from(inventory: Inventory) -> JsonValue {
        JsonValue::from(
            inventory
                .0
                .iter()
                .map(|(&index, &amount)| (format!("{}", index), amount))
                .collect::<HashMap<String, f32>>(),
        )
    }
}

impl From<Region> for JsonValue {
    fn from(region: Region) -> JsonValue {
        object! {
            id: region.id,
            tiles: region.tiles.iter().map(|&tile| usize_to_vec!(tile)).collect::<Vec<Vec<usize>>>(),
            resources: region.resources,
            terrain: region.terrain,
            adjacent_regions: region.adjacent_regions,
            ancestor_race: "Human",
            demographics: object!{Human: 1.0},
            monster: Monster {
                alive: true,
                location: 12,
                inventory: Inventory::default(),
                species: String::from("Dragon"),
                name: String::from("Drew"),
                desc: String::from("Draggin'")
            }
        }
    }
}

impl From<Monster> for JsonValue {
    fn from(monster: Monster) -> JsonValue {
        object! {
            name: monster.name,
            species: monster.species,
            desc: monster.desc,
            inventory: monster.inventory,
            alive: monster.alive,
            location: usize_to_vec!(monster.location)
        }
    }
}

impl From<Snapshot> for JsonValue {
    fn from(snapshot: Snapshot) -> JsonValue {
        object! {
            population: snapshot.population,
            production: snapshot.production,
            imports: snapshot.imports
        }
    }
}

impl From<Npc> for JsonValue {
    fn from(npc: Npc) -> JsonValue {
        object! {
            name: npc.name,
            title: npc.title,
            pos: usize_to_vec!(npc.pos),
            origin: usize_to_vec!(npc.origin),
            birth: npc.birth,
            age: npc.age,
            race: "Human",
            alive: npc.alive,
            skills: object!{},
            inventory: object!{},
            life: array![],
            reputation: 10,
            skills: npc.skills,
        }
    }
}

impl From<City<'_>> for JsonValue {
    fn from(city: City) -> JsonValue {
        object! {
            pos: usize_to_vec!(city.pos),
            name: city.name,
            population: city.population,
            homunculi: 0,
            NPCs: city.npcs,
            data: city.data,
            imports: city.imports,
            production: city.production,
            resources: city.resources,
            economy: city.economy,
            resource_gathering: city.resource_gathering,
            history: array![],
            trade: array![],
            artifacts: array![],
            cultural_values: object!{},
            library: object!{}
        }
    }
}

impl From<Config> for JsonValue {
    fn from(config: Config) -> JsonValue {
        object! {
            GEN_RADIUS: config.gen_radius,
            WORLD_SIZE: vec![config.world_size.0, config.world_size.1],
            COASTAL_CITY_DENSITY: config.coastal_city_density,
            INLAND_CITY_DENSITY: config.inland_city_density,
            PRODUCTION_CONSTANT: config.production_constant,
            POPULATION_CONSTANT: config.population_constant,
            NOTABLE_NPC_THRESHOLD: config.notable_npc_threshold,
            TRADE_VOLUME: config.trade_volume,
            TRADE_QUANTITY: config.trade_quantity,
            ARMY_SIZE: 200,
            ARMY_PARAMETER: 0.7
        }
    }
}

fn to_json(
    region_list: &[Region],
    city_list: HashMap<usize, City>,
    trade_connections: &HashMap<(usize, usize), i32>,
    current_year: u32,
) -> json::JsonValue {
    macro_rules! item_type {
        ($type: ident) => {
            $type::iter()
                .map(|a| (String::from(a.as_ref()), vec![1, 1, 1, 1]))
                .collect::<HashMap<String, Vec<i32>>>()
        };
    }

    json::object! {
        file_type: "save",
        RegionList: region_list,
        CityList: city_list.values().map(|city| JsonValue::from(city.clone())).collect::<Vec<JsonValue>>(),
        trade_connections: trade_connections.iter().map(|((first, second), &strength)| (format!("[{}, {}, {}, {}]", first % CONFIG.world_size.0, first / CONFIG.world_size.0, second % CONFIG.world_size.0, second / CONFIG.world_size.0), strength)).collect::<HashMap<String, i32>>(),
        Biomes: {
            Desert: {
                Resources: {
                    Animal: 0.1, Fish: 0.0, Plant: 0.2, Metal: 0.1, Gemstone: 0.2
                },
                Monsters: ["Worm", "Dragon"],
                Color: [160, 140, 90]
            },
            Forest: {
                Resources: {
                    Animal: 0.3, Fish: 0.0, Plant: 0.9, Metal: 0.1, Gemstone: 0.1
                },
                Monsters: ["Beast"],
                Color: [90, 150, 80]
            },
            Jungle: {
                Resources: {
                    Animal: 0.4, Fish: 0.0, Plant: 1.0, Metal: 0.1, Gemstone: 0.1
                },
                Monsters: ["Beast"],
                Color: [40, 130, 80]
            },
            Mountain: {
                Resources: {
                    Animal: 0.2, Fish: 0.0, Plant: 0.4, Metal: 0.5, Gemstone: 0.4
                },
                Monsters: ["Dragon"],
                Color: [95, 95, 95]
            },
            Ocean: {
                Resources: {},
                Monsters: ["Leviathan"],
                Color: [70, 90, 140]
            },
            Plain: {
                Resources: {
                    Animal: 0.4, Fish: 0.0, Plant: 0.9, Metal: 0.1, Gemstone: 0.1
                },
                Monsters: ["Dragon", "Beast"],
                Color: [120, 140, 80]
            },
            Sea: {
                Resources: {},
                Monsters: ["Worm", "Leviathan"],
                Color: [70, 100, 140]
            }},
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
        current_year: current_year,
        Config: CONFIG.clone()
    }
}

#[derive(Debug, Clone, Copy, AsRefStr, PartialEq, EnumIter)]
enum Terrain {
    Ocean,
    Plain,
    Forest,
    Mountain,
    Desert,
    Jungle,
}

#[derive(Debug, Clone, Copy, AsRefStr, Eq, Hash, PartialEq, EnumIter)]
enum Plant {
    Apple,
    Pepper,
    Pumpkin,
}

impl Default for Plant {
    fn default() -> Self {
        Plant::Apple
    }
}

#[derive(Debug, Clone, Copy, AsRefStr, Eq, Hash, PartialEq, EnumIter)]
enum Metal {
    Iron,
    Copper,
    Gold,
    Silver,
}

impl Default for Metal {
    fn default() -> Self {
        Metal::Iron
    }
}

#[derive(Debug, Clone, Copy, AsRefStr, Eq, Hash, PartialEq, EnumIter)]
enum Gem {
    Diamond,
    Emerald,
    Ruby,
    Agate,
}

impl Default for Gem {
    fn default() -> Self {
        Gem::Diamond
    }
}

#[derive(Debug, Clone, Copy, AsRefStr, Eq, Hash, PartialEq, EnumIter)]
enum Animal {
    Deer,
    Bear,
    Rabbit,
    Wolf,
}

impl Default for Animal {
    fn default() -> Self {
        Animal::Deer
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, EnumIter)]
enum Item {
    Fish,
    Plant(Plant),
    Metal(Metal),
    MetalGood(Metal),
    Gem(Gem),
    CutGem(Gem),
    WildAnimal(Animal),
    TameAnimal(Animal),
    Meat(Animal),
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Fish => write!(f, "Fish"),
            Item::Plant(item) => write!(f, "{}", item.as_ref()),
            Item::Metal(item) => write!(f, "{}", item.as_ref()),
            Item::MetalGood(item) => write!(f, "{} Goods", item.as_ref()),
            Item::Gem(item) => write!(f, "{}", item.as_ref()),
            Item::CutGem(item) => write!(f, "Cut {}", item.as_ref()),
            Item::WildAnimal(item) => write!(f, "Wild {}", item.as_ref()),
            Item::TameAnimal(item) => write!(f, "Tame {}", item.as_ref()),
            Item::Meat(item) => write!(f, "{} Meat", item.as_ref()),
        }
    }
}

static mut ALL_ITEMS : Vec<Item> = Vec::new();

#[derive(Debug, Clone, Default)]
struct Inventory(HashMap<Item, f32>);

impl Inventory {
    fn get(&self, i: Item) -> f32 {
        let result = self.0.get(&i);
        match result {
            None => 0.0,
            Some(&res) => res,
        }
    }

    fn set(&mut self, i: Item, v: f32) {
        self.0.insert(i, v);
    }

    fn add(&mut self, i: Item, v: f32) {
        self.0.insert(i, self.get(i) + v);
    }
}

#[derive(Debug, Clone)]
struct Region {
    id: usize,
    tiles: Vec<usize>,
    resources: Inventory,
    terrain: Terrain,
    adjacent_regions: Vec<usize>,
}

#[derive(Debug, Clone)]
struct Monster {
    alive: bool,
    location: usize,
    inventory: Inventory,
    species: String,
    name: String,
    desc: String,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter, AsRefStr)]
enum Skill {
    Leadership,
    Metalworking,
    Teaching,
    Gemcutting,
    Magic,
    AnimalTraining,
    Adventuring,
    Philosophy,
}

#[derive(Debug, Clone)]
struct Npc {
    name: String,
    title: String,
    pos: usize,
    origin: usize,
    birth: u32,
    age: u32,
    alive: bool,
    skills: HashMap<Skill, u8>,
}

#[derive(Debug, Clone)]
struct Snapshot {
    population: i32,
    production: Inventory,
    imports: Inventory,
}

#[derive(Debug, Clone)]
struct City<'a> {
    name: String,
    pos: usize,
    npcs: Vec<Npc>,
    markov_data: &'a MarkovData,
    population: i32,
    resources: Inventory,
    economy: Inventory,
    resource_gathering: Inventory,
    data: HashMap<String, Snapshot>,
    production: Inventory,
    imports: Inventory,
}

impl City<'_> {
    fn tick(&mut self, rng: &mut ThreadRng, current_year: u32) {
        // Save data
        if current_year % 100 == 0 {
            self.data.insert(
                current_year.to_string(),
                Snapshot {
                    population: self.population,
                    production: std::mem::take(&mut self.production),
                    imports: std::mem::take(&mut self.imports),
                },
            );
        }
        if self.population == 0 {
            return;
        }
        // Produce resources and calculate food
        let mut total_food_resources = 0.0;
        for (&item, &amount) in self.resources.clone().0.iter() {
            let production = inverse_add!(
                (self.population as f32 * 2.0),
                (amount * CONFIG.production_constant)
            )
            .floor();
            self.resources.add(item, production);
            self.production.add(item, production);
            // Deplete non-renewable resources
            match item {
                Item::Metal(_) => self.resource_gathering.add(item, -0.001 * production),
                Item::Gem(_) => self.resource_gathering.add(item, -0.001 * production),
                Item::Plant(_) | Item::Fish | Item::Meat(_) => {
                    total_food_resources += self.resources.get(item)
                }
                _ => (),
            }
        }
        let demand: HashMap<Item, f32> = self
            .resources
            .0
            .iter()
            .map(|(&item, &amount)| {
                (item, {
                    {
                        let mut demand = 0.0;
                        match item {
                            Item::Plant(_) | Item::Fish => {
                                demand += (self.population as f32) * amount / total_food_resources
                            }
                            _ => {}
                        }
                        demand
                    }
                })
            })
            .collect();
        self.economy = Inventory(
            demand
                .iter()
                .map(|(&item, &amount)| {
                    (item, {
                        {
                            let mut price: f32 = 1.0;
                            match item {
                                Item::MetalGood(_) => price *= 4.0,
                                Item::CutGem(_) => price *= 10.0,
                                Item::TameAnimal(_) => price *= 5.0,
                                _ => (),
                            }
                            let exp: f32 = amount / {
                                if self.population as i64 == amount as i64 {
                                    1.0
                                } else {
                                    self.population as f32 - amount
                                }
                            };
                            price * 1.1f32.powf(exp)
                        }
                    })
                })
                .collect(),
        );
        self.resources = Inventory(
            self.resources
                .0
                .iter()
                .map(|(&item, &amount)| (item, (amount - demand[&item]).clamp(0.0, f32::MAX)))
                .collect(),
        );
        let net_food = total_food_resources - self.population as f32;
        self.population += (net_food * CONFIG.population_constant).floor() as i64 as i32;
        if rng.gen::<f32>()
            < (net_food * CONFIG.population_constant)
                - (net_food * CONFIG.population_constant).floor()
        {
            self.population += 1;
        }

        // Tick all living NPCs
        // IMPORTANT: During the loop, the city's npcs list is empty
        let mut npcs = std::mem::take(&mut self.npcs);
        let mut living_npcs: Vec<&mut Npc> = npcs.iter_mut().filter(|npc| npc.alive).collect();
        for npc in &mut living_npcs {
            self.tick_npc(npc, rng);
        }
        if living_npcs.len() < 3 {
            npcs.push(self.generate_npc(rng, current_year))
        }
        self.npcs = npcs;
    }

    fn tick_npc(&mut self, npc: &mut Npc, rng: &mut ThreadRng) {
        npc.age += 1;
        // Die of old age
        if npc.age > 80 {
            npc.alive = false;
            return;
        }
        // Learning / Studying
        if npc.age > 15 {
            let study_choices: Vec<u8> = Skill::iter()
                .map(|skill| *npc.skills.entry(skill).or_insert(0) + 1)
                .collect();
            let study_choice = match WeightedIndex::new(study_choices) {
                Ok(res) => Skill::iter().nth(res.sample(rng)),
                Err(_) => None,
            };
            if let Some(choice) = study_choice {
                if {
                    let luck = rng.gen::<f32>();
                    luck / (1.0 - luck)
                } > (npc.age.pow(2) as f32 * npc.skills[&choice] as f32)
                {
                    npc.skills.insert(choice, npc.skills[&choice] + 1);
                }
            }

            macro_rules! produce_goods {
                ($skill: expr, $material_type: expr, $material: expr => $product: expr) => {
                    let mut prod = npc.skills[$skill] as f32 * 100.0;
                    // Test up to 5 different resources
                    for _ in 1..5 {
                        if prod < 0.0 {
                            break;
                        }
                        let resource = match $material_type.choose(rng) {
                            Some(res) => res,
                            None => break,
                        };
                        let quantity = min(
                            self.resources.get($material(resource).into()) as i64,
                            prod as i64,
                        ) as f32;
                        self.resources.add($material(resource).into(), -quantity);
                        self.resources.add($product(resource).into(), quantity);
                        self.production.add($product(resource).into(), quantity);
                        prod -= quantity;
                    }
                };
            }
            produce_goods!(&Skill::Metalworking, Metal::iter(), &Item::Metal => &Item::MetalGood);
            produce_goods!(&Skill::AnimalTraining, Animal::iter(), &Item::WildAnimal => &Item::TameAnimal);
            produce_goods!(&Skill::AnimalTraining, Animal::iter(), &Item::WildAnimal => &Item::Meat);
            produce_goods!(&Skill::Gemcutting, Gem::iter(), &Item::Gem => &Item::CutGem);
        }
    }

    fn generate_npc(&self, rng: &mut ThreadRng, current_year: u32) -> Npc {
        let name = sample_markov(self.markov_data, rng);
        Npc {
            name,
            title: String::from("citizen"),
            pos: self.pos,
            origin: self.pos,
            age: 0,
            alive: true,
            birth: current_year,
            skills: HashMap::new(),
        }
    }
}

#[derive(Clone)]
struct Config {
    gen_radius: usize,
    world_size: (usize, usize),
    coastal_city_density: f32,
    inland_city_density: f32,
    production_constant: f32,
    population_constant: f32,
    notable_npc_threshold: u8,
    trade_volume: f32,
    trade_quantity: i32,
}

static CONFIG: Config = Config {
    gen_radius: 3,
    world_size: (40, 30),
    coastal_city_density: 0.4,
    inland_city_density: 0.1,
    production_constant: 100.0,
    population_constant: 0.0001,
    notable_npc_threshold: 5,
    trade_volume: 50.0,
    trade_quantity: 20,
};

type MarkovData = (
    Vec<(char, char)>,
    HashMap<(char, char), (Vec<char>, WeightedIndex<u32>)>,
);

fn get_markov_data(strings: &[&str]) -> MarkovData {
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
        let final_counts = intermediate_counts
            .iter()
            .filter_map(|(&k, (chars, weights))| match WeightedIndex::new(weights) {
                Ok(res) => Some((k, (chars.clone(), res))),
                Err(_) => None,
            })
            .collect();
        (starts, final_counts)
    }
}

fn main() {
    let mut rng = thread_rng();
    let markov_data_city: MarkovData = get_markov_data(&[
        "akron;",
        "annapolis;",
        "athens;",
        "atlanta;",
        "austin;",
        "baltimore;",
        "boise;",
        "budapest;",
        "camp hill;",
        "carlisle;",
        "chicago;",
        "chronopolis;",
        "dallas;",
        "denver;",
        "dover;",
        "gotham;",
        "harlem;",
        "harrisburg;",
        "hartford;",
        "honolulu;",
        "indianapolis;",
        "juneau;",
        "lancaster;",
        "london;",
        "los alamos;",
        "los angeles;",
        "madrid;",
        "mechanicsburg;",
        "montgomery;",
        "new york;",
        "orlando;",
        "philadelphia;",
        "phoenix;",
        "pittsburg;",
        "portland;",
        "princeton;",
        "sacramento;",
        "san diego;",
        "scranton;",
        "seattle;",
        "sparta;",
        "springfield;",
        "tallahassee;",
        "trenton;",
        "washington;",
    ]);
    let markov_data_person: MarkovData = get_markov_data(&[
        "abigail;", "ava;", "nova;", "emma;", "maddie;", "natalie;", "abby;", "alethea;", "olivia;",
    ]);
    // println!("{markov_data:?}");
    let (region_map, region_list) = build_region_map(&mut rng);
    let (mut city_list, mut trade_connections) = generate_cities(
        &region_map,
        &region_list,
        &mut rng,
        &markov_data_city,
        &markov_data_person,
    );
    println!("{trade_connections:?}");
    let trade_connections_list: Vec<(usize, usize)> =
        trade_connections.iter().map(|(&k, _v)| k).collect();
    // println!("Region Map: {:?}\nRegion List: {:?}", region_map, region_list);
    for y in 0..CONFIG.world_size.1 {
        for x in 0..CONFIG.world_size.0 {
            if city_list
                .iter()
                .any(|(&pos, _c)| pos == x + y * CONFIG.world_size.0)
            {
                print!("O");
            } else {
                print!(" ");
            }
            let char: char =
                terrain_char!(region_list[region_map[CONFIG.world_size.0 * y + x]].terrain);
            print!("{char}");
        }
        println!();
    }
    let mut current_year: u32 = 0;
    for _ in 0..10000 {
        current_year += 1;
        if current_year % 100 == 0 {
            println!("{current_year}");
        }
        for city in city_list.values_mut() {
            city.tick(&mut rng, current_year);
        }
        for _ in 0..CONFIG.trade_quantity {
            let _ = handle_trade(
                match trade_connections_list.choose(&mut rng) {
                    Some(&res) => res,
                    None => continue,
                },
                &mut city_list,
                &mut trade_connections,
            );
        }
        // println!("{current_year}");
    }
    // println!("{city_list:?}");
    fs::write(
        "./foo.json",
        to_json(&region_list, city_list, &trade_connections, current_year).dump(),
    )
    .expect("Unable to write file");
}

fn sample_markov(markov_data: &MarkovData, rng: &mut ThreadRng) -> String {
    loop {
        if let Some(res) = sample_markov_option(markov_data, rng) {
            return res;
        }
    }
}

fn sample_markov_option(markov_data: &MarkovData, rng: &mut ThreadRng) -> Option<String> {
    let mut result: String = {
        let chars: (char, char) = match markov_data.0.choose(rng) {
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
        result.push(match markov_data.1.get(&ending) {
            Some(result) => match result.0.get(result.1.sample(rng)) {
                Some(&';') => break,
                Some(&c) => c,
                None => break,
            },
            None => break,
        })
    }
    // println!("{result:?}");
    if 5 < result.len() && result.len() < 15 {
        Some(result)
    } else {
        None
    }
}

fn build_region_map(mut rng: &mut ThreadRng) -> (Vec<usize>, Vec<Region>) {
    let mut regions = 0;
    let mut region_map: Vec<Option<usize>> = vec![None; CONFIG.world_size.0 * CONFIG.world_size.1];
    for y in 0..CONFIG.world_size.1 {
        region_map[(y * CONFIG.world_size.0)] = Some(0);
        region_map[(CONFIG.world_size.0 + y * CONFIG.world_size.0 - 1)] = Some(0);
    }
    for x in 0..CONFIG.world_size.0 {
        region_map[x] = Some(0);
        region_map[((CONFIG.world_size.1 - 1) * CONFIG.world_size.0 + x)] = Some(0);
    }
    // println!("{region_map:?}");
    let mut indices: Vec<usize> = (0..(CONFIG.world_size.0 * CONFIG.world_size.1)).collect();
    loop {
        indices.shuffle(&mut rng);
        for index in indices.clone() {
            if match region_map.get(index) {
                Some(res) => res.is_some(),
                None => false,
            } {
                // println!("Index already filled");
                indices.remove(
                    indices
                        .iter()
                        .position(|x| *x == index)
                        .expect("Index somehow gone already??"),
                );
                continue;
            }
            for n in 0..CONFIG.gen_radius {
                let adj: Vec<usize> = get_adj!(&index, n)
                    .iter()
                    .filter_map(|&m| region_map[m])
                    .collect();
                // println!("{adj:?}");
                if adj.is_empty() {
                    // println!("No adjacent non -1");
                    continue;
                }
                region_map[index] = adj.choose(&mut rng).copied();
                // println!("Set Region to {}", region_map[index as usize]);
                break;
            }
            if match region_map.get(index) {
                Some(res) => res.is_none(),
                None => false,
            } {
                // println!("Starting a new region");
                regions += 1;
                region_map[index] = Some(regions);
                indices.remove(
                    indices
                        .iter()
                        .position(|x| *x == index)
                        .expect("Index somehow gone already??"),
                );
            }
        }
        if !indices.iter().any(|&item| region_map[item].is_none()) {
            // println!("Found no -1");
            break;
        }
    }
    let region_map_fixed: Vec<usize> = region_map.iter().map(|&m| m.unwrap_or(0)).collect();
    let mut region_list: Vec<Region> = (0..regions)
        .map(|id| random_region(id + 1, &region_map_fixed, rng, regions))
        .collect();
    region_list.insert(0, {
        let mut base_region = random_region(0, &region_map_fixed, rng, regions);
        base_region.terrain = Terrain::Ocean;
        base_region
    });
    (region_map_fixed, region_list)
}

fn random_region(
    id: usize,
    region_map: &[usize],
    rng: &mut ThreadRng,
    region_count: usize,
) -> Region {
    let tiles: Vec<usize> = (0..(CONFIG.world_size.0 * CONFIG.world_size.1))
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
            Terrain::Plain => {
                // Plain
                (0.2, 0.1, 0.4, 0.9)
            }
            Terrain::Forest => (0.1, 0.2, 0.9, 0.4),
            Terrain::Mountain => (0.9, 0.4, 0.2, 0.1),
            Terrain::Desert => (0.4, 0.9, 0.1, 0.2),
            Terrain::Jungle => (0.1, 0.4, 0.9, 0.2),
            Terrain::Ocean => (0.0, 0.0, 0.0, 0.0),
        };

        let mut resources = Inventory::default();

        macro_rules! run_type {
            ($resource : expr, $resource_item : expr, $resource_names : expr) => {
                for resource_type in $resource_names {
                    if rng.gen::<f32>() < $resource {
                        resources.set(
                            $resource_item(resource_type).into(),
                            rng.gen::<f32>() * $resource + 1.0,
                        );
                    }
                }
            };
        }
        run_type!(metal, Item::Metal, Metal::iter());
        run_type!(gem, Item::Gem, Gem::iter());
        run_type!(plant, Item::Plant, Plant::iter());
        run_type!(animal, Item::WildAnimal, Animal::iter());
        resources.set(Item::Fish, rng.gen::<f32>() * 2.0);
        resources
    };
    Region {
        id,
        tiles: tiles.clone(),
        resources,
        terrain,
        adjacent_regions: (0..region_count)
            .filter(|&neighbor_region| {
                tiles.iter().any(|&tile| {
                    get_adj!(tile, 1)
                        .iter()
                        .any(|&local_region| local_region == neighbor_region)
                })
            })
            .collect(),
    }
}

fn generate_cities<'a>(
    region_map: &[usize],
    region_list: &[Region],
    rng: &mut ThreadRng,
    markov_data: &MarkovData,
    markov_data_person: &'a MarkovData,
) -> (HashMap<usize, City<'a>>, HashMap<(usize, usize), i32>) {
    let mut possible_cities: Vec<usize> = Vec::new();
    for x in 0..region_map.len() {
        if region_list[region_map[x]].terrain == Terrain::Ocean {
            continue;
        }
        if get_adj!(x, 1)
            .iter()
            .any(|&m| region_list[region_map[m]].terrain == Terrain::Ocean)
        {
            if rng.gen::<f32>() > CONFIG.coastal_city_density {
                continue;
            }
        } else if rng.gen::<f32>() > CONFIG.inland_city_density {
            continue;
        }
        possible_cities.push(x);
    }
    let mut actual_cities: Vec<usize> = Vec::new();
    possible_cities.shuffle(rng);
    for x in possible_cities {
        // Discard a city if there's already a city adjacent to it
        if get_adj!(x, 1)
            .iter()
            .any(|&x| actual_cities.iter().any(|&c| x == c))
        {
            continue;
        }
        // println!("{:?} != {:?}", actual_cities, get_adj!(&WORLD_SIZE, x, 1));
        actual_cities.push(x);
    }
    return (
        actual_cities
            .iter()
            .map(|&pos| {
                (
                    pos,
                    City {
                        pos,
                        name: sample_markov(markov_data, rng),
                        npcs: Vec::new(),
                        markov_data: markov_data_person,
                        population: 100,
                        resources: Inventory::default(),
                        economy: Inventory::default(),
                        resource_gathering: Inventory(
                            region_list[region_map[pos]]
                                .resources
                                .0
                                .iter()
                                .map(|(&item, &val)| (item, val + rng.gen::<f32>() * 0.1))
                                .collect(),
                        ),
                        data: HashMap::new(),
                        imports: Inventory::default(),
                        production: Inventory::default(),
                    },
                )
            })
            .collect(),
        {
            // Trade Connections
            let mut trade_connections: HashMap<(usize, usize), i32> = HashMap::new();
            for &start in &actual_cities {
                trade_connections.extend(
                    actual_cities
                        .iter()
                        .filter(|&&end| end > start && distance!(end, start) < 5.0)
                        .map(|&end| ((start, end), 0)),
                )
            }
            trade_connections
        },
    );
}

fn handle_trade(
    route: (usize, usize),
    city_list: &mut HashMap<usize, City>,
    trade_connections: &mut HashMap<(usize, usize), i32>,
) -> Option<()> {
    // immutable references to generate the resource lists
    let first_city = city_list.get(&route.0)?;
    let second_city = city_list.get(&route.1)?;

    let trade_deficit: HashMap<Item, f32> = first_city
        .economy
        .0
        .iter()
        .map(|(&item, &demand)| (item, demand - second_city.economy.get(item)))
        .collect();

    let first_resource = {
        let tup = trade_deficit
            .iter()
            .max_by_key(|(_, &amount)| amount as i64)?;
        (*tup.0, *tup.1)
    };
    let second_resource = {
        let tup = trade_deficit
            .iter()
            .min_by_key(|(_, &amount)| amount as i64)?;
        (*tup.0, *tup.1)
    };

    // mutable references to update the cities' contents. They have to be like this because you can't have two mutable references at the same time
    let first_city = city_list.get_mut(&route.0)?;
    first_city.resources.add(first_resource.0, first_resource.1);
    first_city.imports.add(first_resource.0, first_resource.1);
    first_city
        .resources
        .add(second_resource.0, -second_resource.1);
    first_city
        .imports
        .add(second_resource.0, -second_resource.1);

    let second_city = city_list.get_mut(&route.1)?;
    second_city
        .resources
        .add(first_resource.0, -first_resource.1);
    second_city.imports.add(first_resource.0, -first_resource.1);
    second_city
        .resources
        .add(second_resource.0, second_resource.1);
    second_city
        .imports
        .add(second_resource.0, second_resource.1);

    trade_connections.insert(route, *trade_connections.get(&route).unwrap_or(&0) + 1);
    None
}
