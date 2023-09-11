use crate::sim::{Region, Terrain};
#[allow(clippy::wildcard_imports)]
use crate::*;

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
        region_map[y * config.world_size.0] = Some(0);
        region_map[config.world_size.0 + y * config.world_size.0 - 1] = Some(0);
    }
    for x in 0..config.world_size.0 {
        region_map[x] = Some(0);
        region_map[(config.world_size.1 - 1) * config.world_size.0 + x] = Some(0);
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
        base_region.set_terrain(Terrain::Ocean);
        base_region
    });
    (region_map_fixed, region_list)
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
        if region_list[region_map[x]].terrain() == Terrain::Ocean {
            continue;
        }
        if get_adj(x, 1, config)
            .iter()
            .any(|&m| region_list[region_map[m]].terrain() == Terrain::Ocean)
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
                    City::new(
                        pos,
                        markov_data.sample(rng),
                        Inventory::from(
                            region_list[region_map[pos]]
                                .resources()
                                .iter()
                                .enumerate()
                                .map(|(_, &val)| rng.gen::<f32>().mul_add(0.1, val))
                                .collect::<Vec<_>>(),
                        ),
                        items,
                    ),
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
