use std::collections::HashMap;

use json::{array, object, JsonValue};
use rand::{
    distributions::WeightedIndex, prelude::Distribution, rngs::ThreadRng, seq::SliceRandom, Rng,
};
use strum::IntoEnumIterator;

use crate::{
    distance, get_adj, inverse_add,
    jsonize::{json_array_to_usize, json_int, json_string, Jsonizable},
    magic::MagicSystem,
    mkv::MarkovData,
    mut_loop, usize_to_vec, Config, Items, Npc, Skill,
};

use super::{HistoricalEvent, Inventory, Item, Snapshot};

#[derive(Debug, Clone)]
pub struct City {
    name: String,
    pos: usize,
    npcs: Vec<Npc>,
    population: i32,
    homunculi: i32,
    resources: Inventory,
    economy: Inventory,
    resource_gathering: Inventory,
    data: HashMap<String, Snapshot>,
    production: Inventory,
    imports: Inventory,
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
        let JsonValue::Object(object) = src else { return None; };
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

impl City {
    pub const fn name(&self) -> &String {
        &self.name
    }

    pub const fn pos(&self) -> usize {
        self.pos
    }

    pub const fn data(&self) -> &HashMap<String, Snapshot> {
        &self.data
    }

    pub fn new(pos: usize, name: String, resource_gathering: Inventory, items: &Items) -> Self {
        Self {
            pos,
            name,
            npcs: Vec::new(),
            population: 100,
            homunculi: 0,
            resources: Inventory::default(items),
            economy: Inventory::default(items),
            imports: Inventory::default(items),
            production: Inventory::default(items),
            data: HashMap::new(),
            resource_gathering,
        }
    }

    fn save_snapshot(&mut self, current_year: u32, items: &Items) {
        self.data.insert(
            current_year.to_string(),
            Snapshot {
                population: self.population,
                production: std::mem::replace(&mut self.production, Inventory::default(items)),
                imports: std::mem::replace(&mut self.imports, Inventory::default(items)),
            },
        );
    }

    fn produce_resources(&mut self, config: &Config, items: &Items) {
        for item in 0..items.all.len() {
            let production = {
                let production = inverse_add(
                    self.population as f32 + self.homunculi as f32,
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
                _ => (),
            }
        }
    }

    fn demand(
        resources: &Inventory,
        population: f32,
        total_food_resources: f32,
        items: &Items,
    ) -> Vec<f32> {
        resources
            .iter()
            .enumerate()
            .map(|(item, &amount)| {
                let mut demand = 0.0;
                if items.all.get(item).is_some_and(Item::is_food) {
                    demand += population * amount / total_food_resources;
                }
                demand
            })
            .collect()
    }

    pub fn tick(
        &mut self,
        rng: &mut ThreadRng,
        current_year: u32,
        config: &Config,
        items: &Items,
        magic: &MagicSystem,
        markov_data_npc: &MarkovData,
    ) {
        // Save data
        if current_year % 100 == 0 {
            self.save_snapshot(current_year, items);
        }
        if self.population <= 0 {
            return;
        }
        self.produce_resources(config, items);
        // count food resources
        let mut total_food_resources = 0.0;
        for i in 0..items.all.len() {
            if matches!(items.all[i], Item::Fish | Item::Meat(_) | Item::Plant(_)) {
                total_food_resources += self.resources.get(i);
            }
        }
        // figure out demand for all the items
        let demand = Self::demand(
            &self.resources,
            self.population as f32,
            total_food_resources,
            items,
        );
        // set the price of everything based on demand
        self.economy = Inventory::from(
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
                .collect::<Vec<_>>(),
        );
        // make sure nothing is negative
        for (item, amount) in self.resources.iter_mut().enumerate() {
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
            self.tick_npc(npc, rng, current_year, config, items, magic);
        });
        if living_npcs.len() < 3 {
            npcs.push(self.generate_npc(rng, current_year, markov_data_npc));
        }
        self.npcs = npcs;
    }

    fn get_traveler_options(npc: &Npc, config: &Config, rng: &mut ThreadRng) -> Vec<usize> {
        get_adj(npc.pos, 1, config)
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
            .collect()
    }

    fn tick_npc(
        &mut self,
        npc: &mut Npc,
        rng: &mut ThreadRng,
        current_year: u32,
        config: &Config,
        items: &Items,
        magic: &MagicSystem,
    ) {
        npc.age += 1;
        // Die of old age
        if npc.age > 80 {
            npc.alive = false;
            return;
        }
        // Traveling
        let traveler_options = Self::get_traveler_options(npc, config, rng);
        if npc.pos != npc.origin && !traveler_options.is_empty() {
            // Continue traveling; unwrap is safe as long as traveler_options isn't empty
            npc.pos = *traveler_options.choose(rng).unwrap();
            if npc.pos == npc.origin {
                // Stop traveling
                npc.life.push(HistoricalEvent {
                    time: current_year,
                    description: String::from("stopped traveling"),
                });
                return;
            }
            // if the npc is home
        } else if npc.pos == npc.origin
        // and old enough to travel
            && npc.age > 15
            // and skilled enough in adventuring to feel like it
            && rng.gen::<f32>() * 10.0
                < (*npc.skills.entry(Skill::Adventuring).or_insert(0) as f32 / npc.age as f32)
                // and has somewhere to go
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
            Self::npc_study(rng, npc, current_year);
            self.npc_work(rng, npc, items);
            self.npc_magic_work(rng, npc, magic);
        }
    }

    fn npc_study(rng: &mut ThreadRng, npc: &mut Npc, current_year: u32) {
        let study_choices: Vec<u8> = Skill::iter()
            .map(|skill| *npc.skills.entry(skill).or_insert(0) + 1)
            .collect();
        let study_choice = WeightedIndex::new(study_choices)
            .map_or(None, |res| Skill::iter().nth(res.sample(rng)));
        let Some(choice) = study_choice else { return };
        if {
            let luck = rng.gen::<f32>();
            luck / (1.0 - luck)
        } < (npc.age.pow(2) as f32 * npc.skills[&choice] as f32)
        {
            return;
        }
        *npc.skills.get_mut(&choice).unwrap() += 1;
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

    fn npc_work(&mut self, rng: &mut ThreadRng, npc: &mut Npc, items: &Items) {
        macro_rules! produce_goods {
            ($skill: expr, $material_type: expr, $material: expr => $product: expr) => {
                let mut prod = npc.skills[$skill] as f32 * 100.0;
                // Test up to 5 different resources
                for _ in 1..5 {
                    if prod < 0.0 {
                        break;
                    }
                    let resource = rng.gen_range(0..$material_type.len());
                    let resource_usize = $material(resource as u8).to_index(items).unwrap();
                    let result_usize = $product(resource as u8).to_index(items).unwrap();
                    let quantity =
                        std::cmp::min(self.resources.get(resource_usize) as i64, prod as i64)
                            as f32;
                    self.resources.add(resource_usize, -quantity);
                    self.resources.add(result_usize, quantity);
                    self.production.add(result_usize, quantity);
                    prod -= quantity;
                }
            };
        }
        produce_goods!(&Skill::Metalworking, items.metals, &Item::Metal => &Item::MetalGood);
        produce_goods!(&Skill::AnimalTraining, items.animals, &Item::WildAnimal => &Item::Meat);
        produce_goods!(&Skill::AnimalTraining, items.animals, &Item::WildAnimal => &Item::TameAnimal);
        produce_goods!(&Skill::Gemcutting, items.gems, &Item::Gem => &Item::CutGem);
    }

    fn npc_magic_work(&mut self, rng: &mut ThreadRng, npc: &mut Npc, magic: &MagicSystem) {
        let mut magic_prod = npc.skills[&Skill::Magic] as f32 * 100.0;
        let magic_types: Vec<&crate::magic::Ability> = magic
            .abilities
            .iter()
            .filter(|ability| {
                matches!(
                    ability.ability_type,
                    crate::magic::AbilityType::Homunculus
                        | crate::magic::AbilityType::Youth
                        | crate::magic::AbilityType::Portal
                ) && npc.skills[&Skill::Magic] > ability.min_level
            })
            .collect();
        for _ in 1..5 {
            if magic_prod < 0.0 || magic_types.is_empty() {
                break;
            }
            let &magic_type = magic_types.choose(rng).unwrap();
            let quantity = std::cmp::min(
                (self.resources.get(magic.index.unwrap()) / magic_type.strength as f32) as i64,
                magic_prod as i64,
            ) as f32;
            self.resources.add(magic.index.unwrap(), -quantity);
            match magic_type.ability_type {
                crate::magic::AbilityType::Homunculus => self.homunculi += quantity as i32,
                crate::magic::AbilityType::Youth => npc.age -= quantity as u32,
                crate::magic::AbilityType::Portal => todo!(),
                crate::magic::AbilityType::Combat => {}
            }
            magic_prod -= quantity * magic_type.strength as f32;
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

pub fn handle_trade(
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
