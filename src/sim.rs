mod city;
mod item;
mod logging;
mod world;

use std::collections::HashMap;

pub use city::City;
pub use item::{Inventory, Item, ItemType};
pub use logging::{HistoricalEvent, Snapshot};
pub use world::{Monster, Region, Species, Terrain};

use crate::{Config, Items};

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
                    second_city.economy().get(item) * config.trade_volume
                        / first_city.economy().get(item),
                    first_city.economy().get(item) * config.trade_volume
                        / second_city.economy().get(item),
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
    first_city.import_resource(first_resource.0, first_resource.1);
    first_city.import_resource(second_resource.0, -second_resource.1);

    let second_city = city_list.get_mut(&route.1)?;
    second_city.import_resource(first_resource.0, -first_resource.1);
    second_city.import_resource(second_resource.0, second_resource.1);

    trade_connections.insert(route, *trade_connections.get(&route).unwrap_or(&0) + 1);
    None
}

pub fn get_adj(center: usize, radius: usize, config: &Config) -> Vec<usize> {
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

pub fn distance(a: usize, b: usize, config: &Config) -> f32 {
    ((((a / config.world_size.0) as i32 - (b / config.world_size.0) as i32).pow(2)
        + ((a % config.world_size.0) as i32 - (b % config.world_size.0) as i32).pow(2)) as f32)
        .sqrt()
}

pub fn inverse_add(a: f32, b: f32) -> f32 {
    (a * b) / (a + b)
}

pub fn usize_to_vec(index: usize, config: &Config) -> Vec<usize> {
    vec![index % config.world_size.0, index / config.world_size.0]
}
