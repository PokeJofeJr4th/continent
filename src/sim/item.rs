use std::{
    collections::HashMap,
    slice::{Iter, IterMut},
};

use json::JsonValue;

use crate::{
    jsonize::{json_float, Jsonizable},
    Config, Items,
};

#[derive(Debug, Clone)]
pub struct ItemType {
    pub name: String,
    pub rarity: u8,
    pub abundance: u8,
    pub value: u8,
    pub taming: u8,
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
    pub fn is_food(&self) -> bool {
        matches!(self, Self::Fish | Self::Plant(_) | Self::Meat(_))
    }

    pub fn to_string(self, items: &Items) -> String {
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

    pub fn to_index(self, items: &Items) -> Option<usize> {
        items.all.iter().position(|&m| m == self)
    }
}

#[derive(Debug, Clone)]
pub struct Inventory(Vec<f32>);

impl Inventory {
    pub fn default(items: &Items) -> Self {
        Self(vec![0.0; items.all.len()])
    }

    pub fn get(&self, i: usize) -> f32 {
        match self.0.get(i) {
            None => 0.0,
            Some(&res) => res,
        }
    }

    pub fn set(&mut self, i: usize, v: f32) {
        assert!(i < self.0.len());
        if !v.is_nan() {
            self.0[i] = v;
        }
    }

    pub fn add(&mut self, i: usize, v: f32) {
        assert!(!v.is_nan(), "{i} => {v}");
        self.set(i, self.get(i) + v);
    }

    pub fn iter(&self) -> Iter<'_, f32> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, f32> {
        self.0.iter_mut()
    }
}

impl From<Vec<f32>> for Inventory {
    fn from(value: Vec<f32>) -> Self {
        Self(value)
    }
}

impl Jsonizable for Inventory {
    fn jsonize(&self, _config: &Config, items: &Items) -> JsonValue {
        JsonValue::from(
            self.iter()
                .enumerate()
                .filter_map(|(index, &amount)| {
                    if amount == 0.0 {
                        None
                    } else {
                        Some((items.all[index].to_string(items), amount))
                    }
                })
                .collect::<HashMap<String, f32>>(),
        )
    }

    fn dejsonize(src: &JsonValue, _config: &Config, items: &Items) -> Option<Self> {
        let JsonValue::Object(object) = src else { return None; };
        Some(Self(
            items
                .all
                .iter()
                .map(|item| {
                    object.get(&item.to_string(items)).map_or(0.0, |jsonvalue| {
                        json_float(jsonvalue, 2).unwrap_or_default()
                    })
                })
                .collect(),
        ))
    }
}
