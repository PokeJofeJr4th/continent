#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]

use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs,
    io::{self, Write},
    slice::Iter,
};

use clap::{Parser, Subcommand};
use json::{object, JsonValue};
use magic::MagicSystem;
use rand::{prelude::*, seq::SliceRandom, Rng};
use sim::{handle_trade, City, HistoricalEvent};
// use rayon::prelude::*;
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

mod sim;

mod mkv;
use mkv::{MarkovCollection, MarkovData};

mod jsonize;
use jsonize::SuperJsonizable;

mod worldgen;

mod magic;

mod report;

#[macro_export]
macro_rules! mut_loop {
    ($original_list: expr => for $item: ident in $list: ident $func: expr) => {
        let mut $list = std::mem::take(&mut $original_list);
        for _ in 0..$list.len() {
            // unwrap is safe as long as $func doesn't mutate list
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
            Self::Jungle => vec![Species::Beast, Species::Worm],
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

#[derive(Clone)]
pub struct Items {
    all: Vec<Item>,
    plants: Vec<ItemType>,
    metals: Vec<ItemType>,
    gems: Vec<ItemType>,
    animals: Vec<ItemType>,
}

impl Items {
    fn from_item_types(
        plants: Vec<ItemType>,
        metals: Vec<ItemType>,
        gems: Vec<ItemType>,
        animals: Vec<ItemType>,
    ) -> Self {
        let mut all_items: Vec<Item> = vec![Item::Fish];
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
        Self {
            all: all_items,
            plants,
            metals,
            gems,
            animals,
        }
    }
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
        // self.city_list.par_iter().for_each(|(pos, city)| {
        //     city.tick(
        //         rng,
        //         self.current_year,
        //         &self.config,
        //         &self.items,
        //         &self.magic,
        //         markov_data_npc,
        //     )
        // });
        for city in self.city_list.values_mut() {
            city.tick(
                rng,
                self.current_year,
                &self.config,
                &self.items,
                &self.magic,
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

struct WorldGen {
    config: Config,
    items: Items,
    items_src: Vec<String>,
}

#[derive(Parser, Debug)]
struct Args {
    /// Subcommand
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List files
    List,
    /// Run the simulation
    Run {
        /// Duration in years
        #[arg(short, long, default_value_t = 1000)]
        duration: u32,

        /// File to load from
        #[arg()]
        path: String,

        /// File to save to (doesn't save by default)
        #[arg(short, long)]
        save: Option<String>,

        /// Report to save to (doesn't report by default)
        #[arg(short, long)]
        report: Option<String>,
    },
}

fn cmd_run(
    rng: &mut ThreadRng,
    markov: &MarkovCollection,
    duration: u32,
    path: String,
    save: Option<String>,
    report: Option<String>,
) {
    let Ok(contents) = fs::read_to_string(path) else { return };
    let Ok(src) = json::parse(&contents) else { return };
    let Some(mut world) = World::from_file(&src, rng, markov) else { return };

    simulate_world(&mut world, rng, markov, duration);

    if let Some(savefile) = save {
        fs::write(savefile, world.s_jsonize().dump()).expect("Unable to write file");
    }
    if let Some(reportpath) = report {
        fs::write(reportpath, report::report(&world)).expect("Unable to write report");
    }
}

fn simulate_world(
    world: &mut World,
    rng: &mut ThreadRng,
    markov: &MarkovCollection,
    duration: u32,
) {
    let year_delimiter: u32 = (duration / 100).max(1);

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
    for current_year in 0..duration {
        if current_year % year_delimiter == 0 {
            print!("\x1b[32m\x1b[C█\x1b[D\x1b[0m");
            std::io::stdout().flush().unwrap();
        }
        world.tick(rng, &markov.name);
    }
}

struct WorldFinder {
    stack: Vec<fs::DirEntry>,
}

impl WorldFinder {
    fn new() -> io::Result<Self> {
        let mut stack: Vec<fs::DirEntry> = Vec::new();
        stack.extend(fs::read_dir(env::current_dir()?)?.filter_map(Result::ok));
        Ok(Self { stack })
    }
}

impl Iterator for WorldFinder {
    type Item = fs::DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(entry) = self.stack.pop() {
            let Ok(filetype) = entry.file_type() else {
                continue
            };
            if filetype.is_dir() {
                if let Ok(sub_entry) = fs::read_dir(entry.path()) {
                    self.stack.extend(sub_entry.filter_map(Result::ok));
                }
            } else if filetype.is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap_or_default()
                    == "json"
            {
                let Ok(text) = fs::read_to_string(&entry.path()) else { continue };
                let Ok(src) = json::parse(&text) else { continue };
                if World::s_dejsonize(&src).is_some() || WorldGen::s_dejsonize(&src).is_some() {
                    return Some(entry);
                }
            }
        }
        None
    }
}

fn main() {
    let args: Args = Args::parse();
    let mut rng = thread_rng();

    macro_rules! mkv {
        {$path: expr} => {{
                MarkovData::from_bytes(include_bytes!(concat!("..\\markov\\", $path, ".mkv"))).unwrap()
            }
        }
    }

    let mkv: mkv::MarkovCollection = mkv::MarkovCollection {
        gem: mkv!("gemstone"),
        magic: mkv!("magic"),
        metal: mkv!("metal"),
        monster: mkv!("monster"),
        name: mkv!("name"),
        plant: mkv!("plant"),
    };

    match args.command {
        Commands::List => WorldFinder::new()
            .unwrap()
            .for_each(|file| println!("{}", file.path().display())),
        Commands::Run {
            duration,
            path,
            save,
            report,
        } => cmd_run(&mut rng, &mkv, duration, path, save, report),
    }
}
