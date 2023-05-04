use json::*;
use rand::{distributions::WeightedIndex, prelude::*, seq::SliceRandom, Rng};
use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::slice::Iter;
use std::{fmt, fs, env};
use strum::*;

use Skill::*;
use Terrain::*;

mod mkv;
use mkv::MarkovData;

#[warn(clippy::pedantic)]

macro_rules! mut_loop {
    ($original_list: expr => for $item: ident in $list: ident $func: expr) => {
        let mut $list = std::mem::take(&mut $original_list);
        for _ in 0..$list.len() {
            let $item = $list.pop().unwrap();
            $func
            $list.insert(0, $item);
        }
        $original_list = $list;
    };
}

fn get_adj(center: usize, radius: usize) -> Vec<usize> {
    if radius == 0 {
        vec![
            center + 1,
            center - 1,
            center + CONFIG.world_size.0,
            center - CONFIG.world_size.0,
        ]
    } else {
        let mut adj: Vec<usize> = Vec::new();
        for x in 0..=(2 * radius) {
            for y in 0..=(2 * radius) {
                if (x, y) == (radius, radius) {
                    continue;
                }
                let positive = center + x + y * CONFIG.world_size.0;
                let negative = radius * (1 + CONFIG.world_size.0);
                if negative > positive || (center / CONFIG.world_size.0) + y < radius {
                    continue;
                }
                let adj_index: usize = positive - negative;
                if adj_index < CONFIG.world_size.0 * CONFIG.world_size.1
                    && (adj_index / CONFIG.world_size.0
                        == (center / CONFIG.world_size.0) + y - radius)
                {
                    adj.push(adj_index);
                }
            }
        }
        adj
    }
}

fn distance(a: usize, b: usize) -> f32 {
    ((((a / CONFIG.world_size.0) as i32 - (b / CONFIG.world_size.0) as i32).pow(2)
        + ((a % CONFIG.world_size.0) as i32 - (b % CONFIG.world_size.0) as i32).pow(2)) as f32)
        .sqrt()
}

fn inverse_add(a: f32, b: f32) -> f32 {
    (a * b) / (a + b)
}

fn usize_to_vec(index: usize) -> Vec<usize> {
    vec![index % CONFIG.world_size.0, index / CONFIG.world_size.0]
}

impl From<Inventory> for JsonValue {
    fn from(inventory: Inventory) -> JsonValue {
        JsonValue::from(
            inventory
                .iter()
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
}

impl From<Region> for JsonValue {
    fn from(region: Region) -> JsonValue {
        object! {
            id: region.id,
            tiles: region.tiles.iter().map(|&tile| usize_to_vec(tile)).collect::<Vec<Vec<usize>>>(),
            resources: region.resources,
            terrain: region.terrain.as_ref(),
            adjacent_regions: region.adjacent_regions,
            ancestor_race: "Human",
            demographics: object!{Human: 1.0},
            monster: region.monster
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
            location: usize_to_vec(monster.location)
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
            pos: usize_to_vec(npc.pos),
            origin: usize_to_vec(npc.origin),
            birth: npc.birth,
            age: npc.age,
            race: "Human",
            alive: npc.alive,
            skills: object!{},
            inventory: object!{},
            life: npc.life,
            reputation: 10,
            skills: npc.skills,
        }
    }
}

impl From<HistoricalEvent> for JsonValue {
    fn from(value: HistoricalEvent) -> Self {
        object! {
            Type: "Event",
            Time: value.time,
            Desc: value.description
        }
    }
}

impl From<City<'_>> for JsonValue {
    fn from(city: City) -> JsonValue {
        object! {
            pos: usize_to_vec(city.pos),
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

impl From<Species> for JsonValue {
    fn from(value: Species) -> Self {
        JsonValue::from(value.as_ref())
    }
}

impl From<Terrain> for JsonValue {
    fn from(value: Terrain) -> Self {
        object! {
            Resources: {
                Animal: 0.3, Fish: 0.0, Plant: 0.9, Metal: 0.1, Gemstone: 0.1
            },
            Monsters: value.monster_types(),
            Color: value.color()
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
            Desert: Desert,
            Forest: Forest,
            Jungle: Jungle,
            Mountain: Mountain,
            Ocean: Ocean,
            Plain: Plain},
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

#[derive(Debug, Clone, Copy, AsRefStr, PartialEq, EnumIter)]
enum Species {
    Leviathan,
    Dragon,
    Beast,
    Worm,
}

use Species::*;

impl Terrain {
    fn monster_types(&self) -> Vec<Species> {
        match &self {
            Ocean => vec![Leviathan],
            Plain => vec![Dragon, Beast],
            Forest => vec![Beast],
            Mountain => vec![Dragon],
            Desert => vec![Worm, Dragon],
            Jungle => vec![Beast],
        }
    }

    fn color(&self) -> Vec<u8> {
        match &self {
            Ocean => vec![70, 90, 140],
            Plain => vec![120, 140, 80],
            Forest => vec![90, 150, 80],
            Mountain => vec![96, 96, 96],
            Desert => vec![160, 140, 90],
            Jungle => vec![40, 130, 80],
        }
    }
}

impl fmt::Display for Terrain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Ocean => "\x1b[48;5;18m~",
                Plain => "\x1b[48;5;100m%",
                Forest => "\x1b[48;5;22m♧",
                Mountain => "\x1b[48;5;8m◮",
                Desert => "\x1b[48;5;214m#",
                Jungle => "\x1b[48;5;34m♤",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, AsRefStr, Eq, Hash, PartialEq, EnumIter)]
enum Plant {
    Apple,
    Pepper,
    Pumpkin,
}

#[derive(Debug, Clone, Copy, AsRefStr, Eq, Hash, PartialEq, EnumIter)]
enum Metal {
    Iron,
    Copper,
    Gold,
    Silver,
}

#[derive(Debug, Clone, Copy, AsRefStr, Eq, Hash, PartialEq, EnumIter)]
enum Gem {
    Diamond,
    Emerald,
    Ruby,
    Agate,
}

#[derive(Debug, Clone, Copy, AsRefStr, Eq, Hash, PartialEq, EnumIter)]
enum Animal {
    Deer,
    Bear,
    Rabbit,
    Wolf,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
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

const ITEM_COUNT: usize = 32;

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

impl From<Item> for usize {
    fn from(value: Item) -> Self {
        unsafe { ALL_ITEMS.iter().position(|&m| m == value) }.unwrap()
    }
}

impl From<usize> for Item {
    fn from(value: usize) -> Self {
        assert!(value < ITEM_COUNT);
        unsafe { ALL_ITEMS[value] }
    }
}

static mut ALL_ITEMS: Vec<Item> = Vec::new();

#[derive(Debug, Clone)]
struct Inventory(Vec<f32>);

impl Default for Inventory {
    fn default() -> Self {
        Inventory(vec![0.0; ITEM_COUNT])
    }
}

impl Inventory {
    fn get(&self, i: usize) -> f32 {
        let result = self.0.get(i);
        match result {
            None => 0.0,
            Some(&res) => {
                assert!(!res.is_nan());
                res
            }
        }
    }

    fn set(&mut self, i: usize, v: f32) {
        assert!(i < self.0.len());
        assert!(!v.is_nan());
        self.0[i] = v;
    }

    fn add(&mut self, i: usize, v: f32) {
        self.set(i, self.get(i) + v);
    }

    fn iter(&self) -> Iter<'_, f32> {
        self.0.iter()
    }
}

#[derive(Debug, Clone)]
struct Region {
    id: usize,
    tiles: Vec<usize>,
    resources: Inventory,
    terrain: Terrain,
    adjacent_regions: Vec<usize>,
    monster: Option<Monster>,
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
    life: Vec<HistoricalEvent>,
}

#[derive(Debug, Clone)]
struct HistoricalEvent {
    time: u32,
    description: String,
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
    markov_data: &'a mkv::MarkovData,
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
        for item in 0..ITEM_COUNT {
            let production = {
                let production = inverse_add(
                    self.population as f32 * 2.0,
                    self.resource_gathering.get(item) * CONFIG.production_constant,
                )
                .floor();
                if production.is_nan() {
                    0.0
                } else {
                    production
                }
            };
            self.resources.add(item, production);
            self.production.add(item, production);
            // Deplete non-renewable resources and track food resources
            match item.into() {
                Item::Metal(_) => self.resource_gathering.add(item, -0.001 * production),
                Item::Gem(_) => self.resource_gathering.add(item, -0.001 * production),
                Item::Plant(_) | Item::Fish | Item::Meat(_) => {
                    total_food_resources += self.resources.get(item)
                }
                _ => (),
            }
        }
        let demand: Vec<f32> = self
            .resources
            .iter()
            .enumerate()
            .map(|(item, &amount)| {
                let mut demand = 0.0;
                match item.into() {
                    Item::Plant(_) | Item::Fish => {
                        demand += (self.population as f32) * amount / total_food_resources
                    }
                    _ => {}
                }
                demand
            })
            .collect();
        self.economy = Inventory(
            demand
                .iter()
                .enumerate()
                .map(|(item, &amount)| {
                    let price: f32 = match item.into() {
                        Item::MetalGood(_) => 4.0,
                        Item::CutGem(_) => 10.0,
                        Item::TameAnimal(_) => 5.0,
                        Item::Meat(_) => 2.0,
                        _ => 1.0,
                    };
                    let exp: f32 = amount / {
                        if self.population as i64 == amount as i64 {
                            1.0
                        } else {
                            self.population as f32 - amount
                        }
                    };
                    price * 1.1f32.powf(exp)
                })
                .collect(),
        );
        self.resources = Inventory(
            self.resources
                .iter()
                .enumerate()
                .map(|(item, &amount)| (amount - demand[item]).clamp(0.0, f32::MAX))
                .collect(),
        );
        let net_food = total_food_resources - self.population as f32;

        self.population += {
            let diff = net_food * CONFIG.population_constant;
            diff.floor() as i32 + {
                if rng.gen::<f32>() < (diff - diff.floor()) {
                    1
                } else {
                    0
                }
            }
        };

        // Tick all living NPCs
        // IMPORTANT: During the loop, the city's npcs list is empty
        let mut npcs = std::mem::take(&mut self.npcs);
        let mut living_npcs: Vec<&mut Npc> = npcs.iter_mut().filter(|npc| npc.alive).collect();
        mut_loop!(living_npcs => for npc in list {
            self.tick_npc(npc, rng, current_year);
        });
        if living_npcs.len() < 3 {
            npcs.push(self.generate_npc(rng, current_year))
        }
        self.npcs = npcs;
    }

    fn tick_npc(&mut self, npc: &mut Npc, rng: &mut ThreadRng, current_year: u32) {
        npc.age += 1;
        // Die of old age
        if npc.age > 80 {
            npc.alive = false;
            return;
        }
        // Traveling
        let traveler_options: Vec<usize> = get_adj(npc.pos, 1)
            .iter()
            .filter_map(|&point| {
                let dist = distance(point, npc.origin);
                if dist == 0.0 {
                    if rng.gen::<f32>() < ((50.0 - npc.age as f32) / npc.age as f32) {
                        None
                    } else {
                        Some(point)
                    }
                } else if dist < 10.0 {
                    Some(point)
                } else {
                    None
                }
            })
            .collect();

        if npc.pos != npc.origin && !traveler_options.is_empty() {
            // Continue traveling
            npc.pos = *traveler_options.choose(rng).unwrap();
            if npc.pos == npc.origin {
                // Stop traveling
                npc.life.push(HistoricalEvent {
                    time: current_year,
                    description: String::from("stopped traveling"),
                });
                return;
            }
        } else if npc.pos == npc.origin
            && npc.age > 15
            && rng.gen::<f32>() * 10.0
                < (*npc.skills.entry(Adventuring).or_insert(0) as f32 / npc.age as f32)
            && !traveler_options.is_empty()
        {
            // Begin traveling
            npc.pos = *traveler_options.choose(rng).unwrap();
            npc.life.push(HistoricalEvent {
                time: current_year,
                description: String::from("started traveling"),
            });
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
                    match npc.skills.get(&choice) {
                        Some(2) => npc.life.push(HistoricalEvent {
                            time: current_year,
                            description: String::from("began studying ") + choice.as_ref(),
                        }),
                        Some(5) => npc.life.push(HistoricalEvent {
                            time: current_year,
                            description: String::from("became an apprentice in ") + choice.as_ref(),
                        }),
                        Some(10) => npc.life.push(HistoricalEvent {
                            time: current_year,
                            description: String::from("became a master in ") + choice.as_ref(),
                        }),
                        None => {}
                        Some(_) => {}
                    }
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
            produce_goods!(&Metalworking, Metal::iter(), &Item::Metal => &Item::MetalGood);
            produce_goods!(&AnimalTraining, Animal::iter(), &Item::WildAnimal => &Item::TameAnimal);
            produce_goods!(&AnimalTraining, Animal::iter(), &Item::WildAnimal => &Item::Meat);
            produce_goods!(&Gemcutting, Gem::iter(), &Item::Gem => &Item::CutGem);
        }
    }

    fn generate_npc(&self, rng: &mut ThreadRng, current_year: u32) -> Npc {
        let name = self.markov_data.sample(rng);
        Npc {
            name,
            title: String::from("citizen"),
            pos: self.pos,
            origin: self.pos,
            age: 0,
            alive: true,
            birth: current_year,
            skills: HashMap::new(),
            life: Vec::new(),
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
    coastal_city_density: 0.15,
    inland_city_density: 0.02,
    production_constant: 100.0,
    population_constant: 0.0001,
    notable_npc_threshold: 5,
    trade_volume: 50.0,
    trade_quantity: 20,
};

fn main() {
    unsafe {
        // This is safe because nothing's accessing it yet
        ALL_ITEMS.push(Item::Fish);
        for plant in Plant::iter() {
            ALL_ITEMS.push(Item::Plant(plant));
        }
        for metal in Metal::iter() {
            ALL_ITEMS.push(Item::Metal(metal));
            ALL_ITEMS.push(Item::MetalGood(metal));
        }
        for gem in Gem::iter() {
            ALL_ITEMS.push(Item::Gem(gem));
            ALL_ITEMS.push(Item::CutGem(gem));
        }
        for animal in Animal::iter() {
            ALL_ITEMS.push(Item::WildAnimal(animal));
            ALL_ITEMS.push(Item::TameAnimal(animal));
            ALL_ITEMS.push(Item::Meat(animal));
        }
        assert_eq!(ALL_ITEMS.len(), ITEM_COUNT);
        // println!("{ITEM_COUNT}");
    }

    let mut rng = thread_rng();

    macro_rules! markov_data {
        {$($markov_data: ident from $path: expr),*} => {
            $(
                let mut buf = Vec::new();
                let mut f = File::open($path).unwrap();
            f.read_to_end(&mut buf).unwrap();
            let $markov_data = MarkovData::from_bytes(buf).unwrap();)*
        };
    }

    markov_data! {
        // markov_data_animal from "markov/animal.mkv",
        // markov_data_gemstone from "markov/gemstone.mkv",
        // markov_data_magic from "markov/magic.mkv",
        markov_data_metal from "markov/metal.mkv",
        markov_data_monster from "markov/monster.mkv",
        markov_data_name from "markov/name.mkv"
        // markov_data_plant from "markov/plant.mkv"
    }

    println!("{}", markov_data_metal.sample(&mut rng));
    // println!("{markov_data:?}");
    let (region_map, region_list) = build_region_map(&mut rng, &markov_data_monster);
    let (mut city_list, mut trade_connections) =
        generate_cities(&region_map, &region_list, &mut rng, &markov_data_name);
    // println!("{trade_connections:?}");
    let trade_connections_list: Vec<(usize, usize)> =
        trade_connections.iter().map(|(&k, _v)| k).collect();
    // println!("Region Map: {:?}\nRegion List: {:?}", region_map, region_list);
    for y in 0..CONFIG.world_size.1 {
        for x in 0..CONFIG.world_size.0 {
            print!(
                "{}",
                region_list[region_map[CONFIG.world_size.0 * y + x]].terrain
            );
            if city_list
                .iter()
                .any(|(&pos, _c)| pos == x + y * CONFIG.world_size.0)
            {
                print!("O\x1b[0m");
            } else {
                print!(" \x1b[0m");
            }
        }
        println!();
    }
    // If a year count is provided, use it. Otherwise, just simulate 1000 years
    let year_count: u32 = match env::args().nth(1).map(|arg| arg.parse::<u32>()) {
        Some(Ok(year)) => year,
        _ => 1000,
    };
    for current_year in 0..=year_count {
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
        "./saves/foo.json",
        to_json(&region_list, city_list, &trade_connections, year_count).dump(),
    )
    .expect("Unable to write file");
}

fn build_region_map(
    rng: &mut ThreadRng,
    markov_data_monster: &MarkovData,
) -> (Vec<usize>, Vec<Region>) {
    let mut regions = 0;
    let mut region_map = vec![None; CONFIG.world_size.0 * CONFIG.world_size.1];
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
        indices.shuffle(rng);
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
                let adj: Vec<usize> = get_adj(index, n)
                    .iter()
                    .filter_map(|&m| region_map[m])
                    .collect();
                // println!("{adj:?}");
                if adj.is_empty() {
                    // println!("No adjacent non -1");
                    continue;
                }
                region_map[index] = adj.choose(rng).copied();
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
        .map(|id| random_region(id + 1, &region_map_fixed, rng, regions, markov_data_monster))
        .collect();
    region_list.insert(0, {
        let mut base_region =
            random_region(0, &region_map_fixed, rng, regions, markov_data_monster);
        base_region.terrain = Ocean;
        base_region
    });
    (region_map_fixed, region_list)
}

fn random_region(
    id: usize,
    region_map: &[usize],
    rng: &mut ThreadRng,
    region_count: usize,
    markov_data_monster: &MarkovData,
) -> Region {
    let tiles: Vec<usize> = (0..(CONFIG.world_size.0 * CONFIG.world_size.1))
        .filter(|&i| region_map[i] == id)
        .collect();
    let terrain = {
        let ter_iter = Terrain::iter().collect::<Vec<_>>();
        let ter = ter_iter.choose(rng);
        match ter {
            Some(&terrain) => terrain,
            None => Ocean,
        }
    };
    let resources = {
        let (metal, gem, plant, animal) = match terrain {
            Plain => (0.2, 0.1, 0.4, 0.9),
            Forest => (0.1, 0.2, 0.9, 0.4),
            Mountain => (0.9, 0.4, 0.2, 0.1),
            Desert => (0.4, 0.9, 0.1, 0.2),
            Jungle => (0.1, 0.4, 0.9, 0.2),
            Ocean => (0.0, 0.0, 0.0, 0.0),
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
        resources.set(Item::Fish.into(), rng.gen::<f32>() * 2.0);
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
                    get_adj(tile, 1)
                        .iter()
                        .any(|&local_region| local_region == neighbor_region)
                })
            })
            .collect(),
        monster: {
            let species = *terrain.monster_types().choose(rng).unwrap();
            Some(Monster {
                alive: true,
                location: *tiles.choose(rng).unwrap(),
                inventory: Inventory::default(),
                species: String::from(species.as_ref()),
                name: markov_data_monster.sample(rng),
                desc: {
                    let color = ["red", "blue", "black", "white", "green", "gray"]
                        .choose(rng)
                        .unwrap();
                    match species {
                        Leviathan => format!(
                            "a giant sea creature with {} tentacles, a {}, and {}, {} skin",
                            ((3..=8).choose(rng).unwrap()) * 2,
                            ["chitinous beak", "toothy maw"].choose(rng).unwrap(),
                            ["slimy", "smooth", "rough", "bumpy"].choose(rng).unwrap(),
                            color
                        ),
                        Dragon => format!(
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
                        Beast => {
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
                                        format!("with the head of a {} and", species2)
                                    }
                                },
                                part,
                                species
                            )
                        }
                        Worm => format!(
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
            })
        },
    }
}

fn generate_cities<'a>(
    region_map: &[usize],
    region_list: &[Region],
    rng: &mut ThreadRng,
    markov_data: &'a mkv::MarkovData,
) -> (HashMap<usize, City<'a>>, HashMap<(usize, usize), i32>) {
    let mut possible_cities = Vec::new();
    for x in 0..region_map.len() {
        if region_list[region_map[x]].terrain == Ocean {
            continue;
        }
        if get_adj(x, 1)
            .iter()
            .any(|&m| region_list[region_map[m]].terrain == Ocean)
        {
            if rng.gen::<f32>() > CONFIG.coastal_city_density {
                continue;
            }
        } else if rng.gen::<f32>() > CONFIG.inland_city_density {
            continue;
        }
        possible_cities.push(x);
    }
    let mut actual_cities = Vec::new();
    possible_cities.shuffle(rng);
    for x in possible_cities {
        // Discard a city if there's already a city adjacent to it
        if get_adj(x, 1)
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
                        name: markov_data.sample(rng),
                        npcs: Vec::new(),
                        markov_data,
                        population: 100,
                        resources: Inventory::default(),
                        economy: Inventory::default(),
                        resource_gathering: Inventory(
                            region_list[region_map[pos]]
                                .resources
                                .iter()
                                .enumerate()
                                .map(|(_, &val)| val + rng.gen::<f32>() * 0.1)
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
                        .filter(|&&end| end > start && distance(end, start) < 5.0)
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

    let (first_city_supply, second_city_supply): (Vec<f32>, Vec<f32>) = {
        (0..ITEM_COUNT)
            .map(|item| {
                (
                    second_city.economy.get(item) * CONFIG.trade_volume
                        / first_city.economy.get(item),
                    first_city.economy.get(item) * CONFIG.trade_volume
                        / second_city.economy.get(item),
                )
            })
            .unzip()
    };

    let first_resource = {
        let tup = first_city_supply
            .iter()
            .enumerate()
            .min_by_key(|(_, &amount)| amount as i64)?;
        (tup.0, tup.1.floor())
    };
    let second_resource = {
        let tup = second_city_supply
            .iter()
            .enumerate()
            .min_by_key(|(_, &amount)| amount as i64)?;
        (tup.0, *tup.1)
    };

    // mutable references to update the cities' contents.
    // They have to be like this because you can't have two mutable references at the same time
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
