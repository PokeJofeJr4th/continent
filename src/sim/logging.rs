use json::{object, JsonValue};

use crate::{
    jsonize::{json_int, json_string, Jsonizable, SuperJsonizable},
    Config, Inventory, Items,
};

#[derive(Debug, Clone)]
pub struct HistoricalEvent {
    pub time: u32,
    pub description: String,
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
        let JsonValue::Object(object) = src else { return None };
        Some(Self {
            time: json_int(object.get("Time")?)? as u32,
            description: json_string(object.get("Desc")?)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub population: i32,
    pub production: Inventory,
    pub imports: Inventory,
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
        let JsonValue::Object(object) = src else { return None };
        Some(Self {
            population: json_int(object.get("population")?)?,
            production: Inventory::dejsonize(object.get("production")?, config, items)?,
            imports: Inventory::dejsonize(object.get("imports")?, config, items)?,
        })
    }
}
