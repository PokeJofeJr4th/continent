#[allow(clippy::wildcard_imports)]
use crate::*;

impl World {
    pub fn gen(rng: &mut ThreadRng, markov: &MarkovCollection) -> Self {
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
        let magic = MagicSystem::gen(rng, markov);
        match &magic.material_type {
            magic::MaterialType::Plant => plants.push(magic.material.clone()),
            magic::MaterialType::Gem => gems.push(magic.material.clone()),
            magic::MaterialType::Metal => metals.push(magic.material.clone()),
        };
        let items = Items::from_item_types(plants, metals, gems, animals);
        let config = Config::default();
        let (region_map, region_list) = build_region_map(rng, &markov.monster, &config, &items);
        let (city_list, trade_connections) = generate_cities(
            &region_map,
            &region_list,
            rng,
            &markov.name,
            &config,
            &items,
        );
        let trade_connections_list: Vec<(usize, usize)> =
            trade_connections.iter().map(|(&k, _v)| k).collect();
        Self {
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
    }
}

impl WorldGen {
    pub fn sample(&self, rng: &mut ThreadRng, markov: &MarkovCollection) -> World {
        let magic = MagicSystem::gen(rng, markov);
        let Items {
            all: _,
            mut plants,
            mut metals,
            mut gems,
            animals,
        } = self.items.clone();
        match &magic.material_type {
            magic::MaterialType::Plant => &mut plants,
            magic::MaterialType::Gem => &mut gems,
            magic::MaterialType::Metal => &mut metals,
        }
        .push(magic.material.clone());
        let items = Items::from_item_types(plants, metals, gems, animals);
        let (region_map, region_list) =
            build_region_map(rng, &markov.monster, &self.config, &items);
        let (city_list, trade_connections) = generate_cities(
            &region_map,
            &region_list,
            rng,
            &markov.name,
            &self.config,
            &items,
        );
        let trade_connections_list: Vec<(usize, usize)> =
            trade_connections.iter().map(|(&k, _v)| k).collect();
        World {
            config: self.config,
            current_year: 0,
            region_map,
            region_list,
            city_list,
            trade_connections,
            trade_connections_list,
            items,
            magic,
        }
    }
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
            Region::gen(
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
        let mut base_region = Region::gen(
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

impl Region {
    fn gen(
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

impl Monster {
    fn gen(
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
        }
    }
}

fn generate_cities(
    region_map: &[usize],
    region_list: &[Region],
    rng: &mut ThreadRng,
    markov_data: &MarkovData,
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
                        homunculi: 0,
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
