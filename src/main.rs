#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::too_many_lines
)]

use std::cmp::min;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::slice::Iter;

use clap::Parser;
use json::{array, object, JsonValue};
use magic::MagicSystem;
use rand::{distributions::WeightedIndex, prelude::*, seq::SliceRandom, Rng};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

mod mkv;
use mkv::MarkovData;

mod jsonize;
use jsonize::SuperJsonizable;

mod magic;

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

fn get_adj(center: usize, radius: usize, config: &Config) -> Vec<usize> {
    if radius == 0 {
        vec![
            center + 1,
            center - 1,
            center + config.world_size.0,
            center - config.world_size.0,
        ]
    } else {
        let mut adj: Vec<usize> = Vec::new();
        for x in 0..=(2 * radius) {
            for y in 0..=(2 * radius) {
                if (x, y) == (radius, radius) {
                    continue;
                }
                let positive = center + x + y * config.world_size.0;
                let negative = radius * (1 + config.world_size.0);
                if negative > positive || (center / config.world_size.0) + y < radius {
                    continue;
                }
                let adj_index: usize = positive - negative;
                if adj_index < config.world_size.0 * config.world_size.1
                    && (adj_index / config.world_size.0
                        == (center / config.world_size.0) + y - radius)
                {
                    adj.push(adj_index);
                }
            }
        }
        adj
    }
}

fn distance(a: usize, b: usize, config: &Config) -> f32 {
    ((((a / config.world_size.0) as i32 - (b / config.world_size.0) as i32).pow(2)
        + ((a % config.world_size.0) as i32 - (b % config.world_size.0) as i32).pow(2)) as f32)
        .sqrt()
}

fn inverse_add(a: f32, b: f32) -> f32 {
    (a * b) / (a + b)
}

fn usize_to_vec(index: usize, config: &Config) -> Vec<usize> {
    vec![index % config.world_size.0, index / config.world_size.0]
}

#[derive(Debug, Clone, Copy, AsRefStr, PartialEq, Eq, EnumIter)]
pub enum Terrain {
    Ocean,
    Plain,
    Forest,
    Mountain,
    Desert,
    Jungle,
}

#[derive(Debug, Clone, Copy, AsRefStr, PartialEq, Eq, EnumIter)]
pub enum Species {
    Leviathan,
    Dragon,
    Beast,
    Worm,
}

impl Terrain {
    fn monster_types(self) -> Vec<Species> {
        match self {
            Self::Ocean => vec![Species::Leviathan],
            Self::Plain => vec![Species::Dragon, Species::Beast],
            Self::Forest => vec![Species::Beast],
            Self::Mountain => vec![Species::Dragon],
            Self::Desert => vec![Species::Worm, Species::Dragon],
            Self::Jungle => vec![Species::Beast],
        }
    }

    fn color(self) -> Vec<u8> {
        match self {
            Self::Ocean => vec![70, 90, 140],
            Self::Plain => vec![120, 140, 80],
            Self::Forest => vec![90, 150, 80],
            Self::Mountain => vec![96, 96, 96],
            Self::Desert => vec![160, 140, 90],
            Self::Jungle => vec![40, 130, 80],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ItemType {
    name: String,
    rarity: u8,
    abundance: u8,
    value: u8,
    taming: u8,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Item {
    Fish,
    Plant(u8),
    Metal(u8),
    MetalGood(u8),
    Gem(u8),
    CutGem(u8),
    WildAnimal(u8),
    TameAnimal(u8),
    Meat(u8),
}

impl Item {
    fn to_string(self, items: &Items) -> String {
        match self {
            Self::Fish => String::from("Fish"),
            Self::Plant(item) => items.plants[item as usize].name.clone(),
            Self::Metal(item) => items.metals[item as usize].name.clone(),
            Self::MetalGood(item) => format!("{} Goods", items.metals[item as usize].name),
            Self::Gem(item) => items.gems[item as usize].name.clone(),
            Self::CutGem(item) => format!("Cut {}", items.gems[item as usize].name),
            Self::WildAnimal(item) => format!("Wild {}", items.animals[item as usize].name),
            Self::TameAnimal(item) => format!("Tame {}", items.animals[item as usize].name),
            Self::Meat(item) => format!("{} Meat", items.animals[item as usize].name),
        }
    }

    fn to_index(self, items: &Items) -> Option<usize> {
        items.all.iter().position(|&m| m == self)
    }
}

#[derive(Debug, Clone)]
pub struct Inventory(Vec<f32>);

impl Inventory {
    fn default(items: &Items) -> Self {
        Self(vec![0.0; items.all.len()])
    }

    fn get(&self, i: usize) -> f32 {
        match self.0.get(i) {
            None => 0.0,
            Some(&res) => res,
        }
    }

    fn set(&mut self, i: usize, v: f32) {
        assert!(i < self.0.len());
        if !v.is_nan() {
            self.0[i] = v;
        }
    }

    fn add(&mut self, i: usize, v: f32) {
        assert!(!v.is_nan(), "{i} => {v}");
        self.set(i, self.get(i) + v);
    }

    fn iter(&self) -> Iter<'_, f32> {
        self.0.iter()
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

#[derive(Debug, Clone)]
pub struct Monster {
    alive: bool,
    location: usize,
    inventory: Inventory,
    species: String,
    name: String,
    desc: String,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter, AsRefStr)]
pub enum Skill {
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
pub struct Npc {
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
pub struct HistoricalEvent {
    time: u32,
    description: String,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    population: i32,
    production: Inventory,
    imports: Inventory,
}

#[derive(Debug, Clone)]
pub struct City {
    name: String,
    pos: usize,
    npcs: Vec<Npc>,
    population: i32,
    resources: Inventory,
    economy: Inventory,
    resource_gathering: Inventory,
    data: HashMap<String, Snapshot>,
    production: Inventory,
    imports: Inventory,
}

impl City {
    fn tick(
        &mut self,
        rng: &mut ThreadRng,
        current_year: u32,
        config: &Config,
        items: &Items,
        markov_data_npc: &MarkovData,
    ) {
        // Save data
        if current_year % 100 == 0 {
            self.data.insert(
                current_year.to_string(),
                Snapshot {
                    population: self.population,
                    production: std::mem::replace(&mut self.production, Inventory::default(items)),
                    imports: std::mem::replace(&mut self.imports, Inventory::default(items)),
                },
            );
        }
        if self.population <= 0 {
            return;
        }
        // Produce resources and calculate food
        let mut total_food_resources = 0.0;
        for item in 0..items.all.len() {
            let production = {
                let production = inverse_add(
                    self.population as f32 * 1.0,
                    self.resource_gathering.get(item) * config.production_constant,
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
            match items.all.get(item) {
                Some(Item::Metal(_) | Item::Gem(_)) => {
                    self.resource_gathering
                        .add(item, -config.mineral_depletion * production);
                }
                Some(Item::Plant(_) | Item::Fish | Item::Meat(_)) => {
                    total_food_resources += self.resources.get(item);
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
                if let Some(Item::Plant(_) | Item::Fish | Item::Meat(_)) = items.all.get(item) {
                    demand += (self.population as f32) * amount / total_food_resources;
                }
                demand
            })
            .collect();
        self.economy = Inventory(
            demand
                .iter()
                .enumerate()
                .map(|(item, &amount)| {
                    let price: f32 = match items.all.get(item) {
                        Some(Item::MetalGood(_)) => 4.0,
                        Some(Item::CutGem(_)) => 10.0,
                        Some(Item::TameAnimal(_)) => 5.0,
                        Some(Item::Meat(_)) => 2.0,
                        _ => 1.0,
                    };
                    let exp: f32 = amount / { (self.population as f32 - amount).exp() };
                    let val = price * 1.1f32.powf(exp);
                    if val.is_nan() {
                        0.0
                    } else {
                        val
                    }
                })
                .collect(),
        );
        for (item, amount) in self.resources.0.iter_mut().enumerate() {
            *amount = (*amount - demand[item]).clamp(0.0, f32::MAX);
        }
        let net_food = total_food_resources - self.population as f32;

        // At most, half of people die and 2% are born
        self.population += {
            let diff = net_food * config.population_constant;
            diff.floor() as i32 + i32::from(rng.gen::<f32>() < (diff - diff.floor()))
        }
        .clamp(-self.population / 2, self.population / 50);

        // Tick all living NPCs
        // IMPORTANT: During the loop, the city's npcs list is empty
        let mut npcs = std::mem::take(&mut self.npcs);
        let mut living_npcs: Vec<&mut Npc> = npcs.iter_mut().filter(|npc| npc.alive).collect();
        mut_loop!(living_npcs => for npc in list {
            self.tick_npc(npc, rng, current_year, config, items);
        });
        if living_npcs.len() < 3 {
            npcs.push(self.generate_npc(rng, current_year, markov_data_npc));
        }
        self.npcs = npcs;
    }

    fn tick_npc(
        &mut self,
        npc: &mut Npc,
        rng: &mut ThreadRng,
        current_year: u32,
        config: &Config,
        items: &Items,
    ) {
        npc.age += 1;
        // Die of old age
        if npc.age > 80 {
            npc.alive = false;
            return;
        }
        // Traveling
        let traveler_options: Vec<usize> = get_adj(npc.pos, 1, config)
            .iter()
            .filter_map(|&point| {
                let dist = distance(point, npc.origin, config);
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
                < (*npc.skills.entry(Skill::Adventuring).or_insert(0) as f32 / npc.age as f32)
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
            let study_choice = WeightedIndex::new(study_choices)
                .map_or(None, |res| Skill::iter().nth(res.sample(rng)));
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
                        _ => {}
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
                        let resource = rng.gen_range(0..$material_type.len());
                        let res_usize = $material(resource as u8).to_index(items).unwrap();
                        let quantity =
                            min(self.resources.get(res_usize) as i64, prod as i64) as f32;
                        self.resources.add(res_usize, -quantity);
                        self.resources.add(res_usize, quantity);
                        self.production.add(res_usize, quantity);
                        prod -= quantity;
                    }
                };
            }
            produce_goods!(&Skill::Metalworking, items.metals, &Item::Metal => &Item::MetalGood);
            produce_goods!(&Skill::AnimalTraining, items.animals, &Item::WildAnimal => &Item::Meat);
            produce_goods!(&Skill::AnimalTraining, items.animals, &Item::WildAnimal => &Item::TameAnimal);
            produce_goods!(&Skill::Gemcutting, items.gems, &Item::Gem => &Item::CutGem);
        }
    }

    fn generate_npc(
        &self,
        rng: &mut ThreadRng,
        current_year: u32,
        markov_data_npc: &MarkovData,
    ) -> Npc {
        let name = markov_data_npc.sample(rng);
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

#[derive(Clone, Copy)]
pub struct Config {
    gen_radius: usize,
    world_size: (usize, usize),
    coastal_city_density: f32,
    inland_city_density: f32,
    production_constant: f32,
    population_constant: f32,
    mineral_depletion: f32,
    notable_npc_threshold: u8,
    trade_volume: f32,
    trade_quantity: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gen_radius: 3,
            world_size: (40, 30),
            coastal_city_density: 0.15,
            inland_city_density: 0.02,
            production_constant: 60.0,
            population_constant: 0.0001,
            mineral_depletion: 0.00001,
            notable_npc_threshold: 5,
            trade_volume: 50.0,
            trade_quantity: 20,
        }
    }
}

pub struct Items {
    all: Vec<Item>,
    plants: Vec<ItemType>,
    metals: Vec<ItemType>,
    gems: Vec<ItemType>,
    animals: Vec<ItemType>,
}

pub struct World {
    config: Config,
    current_year: u32,
    region_map: Vec<usize>,
    region_list: Vec<Region>,
    city_list: HashMap<usize, City>,
    trade_connections: HashMap<(usize, usize), i32>,
    trade_connections_list: Vec<(usize, usize)>,
    items: Items,
    magic: MagicSystem,
}

impl World {
    fn tick(&mut self, rng: &mut ThreadRng, markov_data_npc: &MarkovData) {
        for city in self.city_list.values_mut() {
            city.tick(
                rng,
                self.current_year,
                &self.config,
                &self.items,
                markov_data_npc,
            );
        }
        for _ in 0..self.config.trade_quantity {
            let _ = handle_trade(
                match self.trade_connections_list.choose(rng) {
                    Some(&res) => res,
                    None => continue,
                },
                &mut self.city_list,
                &mut self.trade_connections,
                &self.config,
                &self.items,
            );
        }
        self.current_year += 1;
    }
}

#[derive(Parser, Debug)]
struct Args {
    // duration to run (default 1000)
    #[arg(short, long, default_value_t = 1000)]
    duration: u32,

    // load from file (default don't)
    #[arg(short, long)]
    path: String,

    // save to file (default foo.json)
    #[arg(short, long)]
    save: String,
}

fn main() {
    let mut rng = thread_rng();

    macro_rules! markov_data {
        {$($markov_data: ident from $path: expr),*} => {
            $(
                let mut buf = Vec::new();
                let mut f = File::open($path).unwrap();
                f.read_to_end(&mut buf).unwrap();
                let $markov_data = MarkovData::from_bytes(&buf).unwrap();)*
            };
        }

    markov_data! {
        // markov_data_animal from "markov/animal.mkv",
        markov_data_gemstone from "markov/gemstone.mkv",
        markov_data_magic from "markov/magic.mkv",
        markov_data_metal from "markov/metal.mkv",
        markov_data_monster from "markov/monster.mkv",
        markov_data_name from "markov/name.mkv",
        markov_data_plant from "markov/plant.mkv"
    }

    let args: Args = Args::parse();

    let year_delimiter: u32 = (args.duration / 100).max(1);

    let mut world = {
        fs::read_to_string(args.path).map_or(None, |contents| {
            json::parse(&contents).map_or(None, |jsonvalue| {
                World::s_dejsonize(&jsonvalue)
            })
        })
    }
    .unwrap_or({
        let mut all_items = vec![Item::Fish];
        let mut plants: Vec<ItemType> = ["Apple", "Pumpkin"]
            .iter()
            .map(|&s| ItemType {
                name: String::from(s),
                rarity: 2,
                abundance: 4,
                value: 1,
                taming: 0,
            })
            .collect();
        let mut metals: Vec<ItemType> = ["Iron", "Gold"]
            .iter()
            .map(|&s| ItemType {
                name: String::from(s),
                rarity: 4,
                abundance: 5,
                value: 4,
                taming: 0,
            })
            .collect();
        let mut gems: Vec<ItemType> = ["Diamond", "Ruby"]
            .iter()
            .map(|&s| ItemType {
                name: String::from(s),
                rarity: 8,
                abundance: 2,
                value: 6,
                taming: 0,
            })
            .collect();
        let animals: Vec<ItemType> = ["Deer", "Wolf"]
            .iter()
            .map(|&s| ItemType {
                name: String::from(s),
                rarity: 5,
                abundance: 6,
                value: 2,
                taming: 4,
            })
            .collect();
        let magic = MagicSystem::gen(
            &mut rng,
            &markov_data_magic,
            &markov_data_gemstone,
            &markov_data_metal,
            &markov_data_plant,
        );
        match &magic.material_type {
            magic::MaterialType::Plant => plants.push(magic.material.clone()),
            magic::MaterialType::Gem => gems.push(magic.material.clone()),
            magic::MaterialType::Metal => metals.push(magic.material.clone()),
        };
        for plant in 0..plants.len() {
            all_items.push(Item::Plant(plant as u8));
        }
        for metal in 0..metals.len() {
            all_items.push(Item::Metal(metal as u8));
            all_items.push(Item::MetalGood(metal as u8));
        }
        for gem in 0..gems.len() {
            all_items.push(Item::Gem(gem as u8));
            all_items.push(Item::CutGem(gem as u8));
        }
        for animal in 0..animals.len() {
            all_items.push(Item::WildAnimal(animal as u8));
            all_items.push(Item::TameAnimal(animal as u8));
            all_items.push(Item::Meat(animal as u8));
        }
        let items = Items {
            all: all_items,
            plants,
            metals,
            gems,
            animals,
        };
        let config = Config::default();
        let (region_map, region_list) =
            build_region_map(&mut rng, &markov_data_monster, &config, &items);
        let (city_list, trade_connections) = generate_cities(
            &region_map,
            &region_list,
            &mut rng,
            &markov_data_name,
            &config,
            &items,
        );
        let trade_connections_list: Vec<(usize, usize)> =
            trade_connections.iter().map(|(&k, _v)| k).collect();
        World {
            config,
            magic,
            current_year: 0,
            region_map,
            region_list,
            city_list,
            trade_connections,
            trade_connections_list,
            items,
        }
    });

    for y in 0..world.config.world_size.1 {
        for x in 0..world.config.world_size.0 {
            print!(
                "{}",
                match world.region_list[world.region_map[world.config.world_size.0 * y + x]].terrain
                {
                    Terrain::Ocean => "\x1b[48;5;18m~",
                    Terrain::Plain => "\x1b[48;5;100m%",
                    Terrain::Forest => "\x1b[48;5;22m♧",
                    Terrain::Mountain => "\x1b[48;5;8m◮",
                    Terrain::Desert => "\x1b[48;5;214m#",
                    Terrain::Jungle => "\x1b[48;5;34m♤",
                }
            );
            if world
                .city_list
                .iter()
                .any(|(&pos, _c)| pos == x + y * world.config.world_size.0)
            {
                print!("O\x1b[0m");
            } else {
                print!(" \x1b[0m");
            }
        }
        println!();
    }
    println!(
        "{}",
        String::from("╔")
            + &"═".repeat(100)
            + "╗\n║"
            + &" ".repeat(100)
            + "║\n╚"
            + &"═".repeat(100)
            + "╝\x1b[2F"
    );
    for current_year in 0..args.duration {
        if current_year % year_delimiter == 0 {
            print!("\x1b[32m\x1b[C█\x1b[D\x1b[0m");
            std::io::stdout().flush().unwrap();
        }
        world.tick(&mut rng, &markov_data_name);
    }
    fs::write(args.save, world.s_jsonize().dump()).expect("Unable to write file");
}

fn build_region_map(
    rng: &mut ThreadRng,
    markov_data_monster: &MarkovData,
    config: &Config,
    items: &Items,
) -> (Vec<usize>, Vec<Region>) {
    let mut regions = 0;
    let mut region_map = vec![None; config.world_size.0 * config.world_size.1];
    for y in 0..config.world_size.1 {
        region_map[(y * config.world_size.0)] = Some(0);
        region_map[(config.world_size.0 + y * config.world_size.0 - 1)] = Some(0);
    }
    for x in 0..config.world_size.0 {
        region_map[x] = Some(0);
        region_map[((config.world_size.1 - 1) * config.world_size.0 + x)] = Some(0);
    }
    let mut indices: Vec<usize> = (0..(config.world_size.0 * config.world_size.1)).collect();
    loop {
        indices.shuffle(rng);
        for index in indices.clone() {
            if region_map.get(index).map_or(false, Option::is_some) {
                indices.remove(
                    indices
                        .iter()
                        .position(|x| *x == index)
                        .expect("Index somehow gone already??"),
                );
                continue;
            }
            for n in 0..config.gen_radius {
                let adj: Vec<usize> = get_adj(index, n, config)
                    .iter()
                    .filter_map(|&m| region_map[m])
                    .collect();
                if adj.is_empty() {
                    continue;
                }
                region_map[index] = adj.choose(rng).copied();
                break;
            }
            if region_map.get(index).map_or(false, Option::is_none) {
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
            break;
        }
    }
    let region_map_fixed: Vec<usize> = region_map.iter().map(|&m| m.unwrap_or(0)).collect();
    let mut region_list: Vec<Region> = (0..regions)
        .map(|id| {
            random_region(
                id + 1,
                &region_map_fixed,
                rng,
                regions,
                markov_data_monster,
                config,
                items,
            )
        })
        .collect();
    region_list.insert(0, {
        let mut base_region = random_region(
            0,
            &region_map_fixed,
            rng,
            regions,
            markov_data_monster,
            config,
            items,
        );
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
    markov_data_monster: &MarkovData,
    config: &Config,
    items: &Items,
) -> Region {
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
    Region {
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
        monster: {
            let species = *terrain.monster_types().choose(rng).unwrap();
            Some(Monster {
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
                            ((3..=8).choose(rng).unwrap()) * 2,
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
            })
        },
    }
}

fn generate_cities(
    region_map: &[usize],
    region_list: &[Region],
    rng: &mut ThreadRng,
    markov_data: &mkv::MarkovData,
    config: &Config,
    items: &Items,
) -> (HashMap<usize, City>, HashMap<(usize, usize), i32>) {
    let mut possible_cities = Vec::new();
    for x in 0..region_map.len() {
        if region_list[region_map[x]].terrain == Terrain::Ocean {
            continue;
        }
        if get_adj(x, 1, config)
            .iter()
            .any(|&m| region_list[region_map[m]].terrain == Terrain::Ocean)
        {
            if rng.gen::<f32>() > config.coastal_city_density {
                continue;
            }
        } else if rng.gen::<f32>() > config.inland_city_density {
            continue;
        }
        possible_cities.push(x);
    }
    let mut actual_cities = Vec::new();
    possible_cities.shuffle(rng);
    for x in possible_cities {
        // Discard a city if there's already a city adjacent to it
        if get_adj(x, 1, config)
            .iter()
            .any(|&x| actual_cities.iter().any(|&c| x == c))
        {
            continue;
        }
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
                        population: 100,
                        resources: Inventory::default(items),
                        economy: Inventory::default(items),
                        resource_gathering: Inventory(
                            region_list[region_map[pos]]
                                .resources
                                .iter()
                                .enumerate()
                                .map(|(_, &val)| rng.gen::<f32>().mul_add(0.1, val))
                                .collect(),
                        ),
                        data: HashMap::new(),
                        imports: Inventory::default(items),
                        production: Inventory::default(items),
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
                        .filter(|&&end| end > start && distance(end, start, config) < 5.0)
                        .map(|&end| ((start, end), 0)),
                );
            }
            trade_connections
        },
    );
}

fn handle_trade(
    route: (usize, usize),
    city_list: &mut HashMap<usize, City>,
    trade_connections: &mut HashMap<(usize, usize), i32>,
    config: &Config,
    items: &Items,
) -> Option<()> {
    // immutable references to generate the resource lists
    let first_city = city_list.get(&route.0)?;
    let second_city = city_list.get(&route.1)?;

    let (first_city_supply, second_city_supply): (Vec<f32>, Vec<f32>) = {
        (0..items.all.len())
            .map(|item| {
                (
                    second_city.economy.get(item) * config.trade_volume
                        / first_city.economy.get(item),
                    first_city.economy.get(item) * config.trade_volume
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
        (tup.0, tup.1.floor())
    };

    if first_resource.1.is_nan()
        || second_resource.1.is_nan()
        || first_resource.1 <= 0.0
        || second_resource.1 <= 0.0
    {
        return None;
    }

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
