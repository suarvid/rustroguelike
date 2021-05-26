use specs::prelude::*;
use crate::{CombatStats, Consumable, InflictsDamage, Map, ProvidesHealing, SufferDamage, WantsToDropItem, WantsToUseItem, map, AreaOfEffect, Confusion};

use super::{WantsToPickUpItem, Name, Item, InBackpack, Position, gamelog::GameLog};

pub struct InventorySystem {}

impl<'a> System<'a> for InventorySystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        WriteStorage<'a, WantsToPickUpItem>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, InBackpack>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, mut wants_pickup, mut positions, names, mut backpack) = data;

        for pickup in wants_pickup.join() {
            positions.remove(pickup.item);
            backpack.insert(pickup.item, InBackpack { owner: pickup.collected_by }).expect("Unable to insert into backpack");

            if pickup.collected_by == *player_entity {
                gamelog.entries.push(format!("You pick up the {}.", names.get(pickup.item).unwrap().name));
            }
        }
        wants_pickup.clear();
    }
}


pub struct ItemUseSystem {}

impl<'a> System<'a> for ItemUseSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadExpect<'a, Map>,
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToUseItem>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, Consumable>,
        ReadStorage<'a, ProvidesHealing>,
        ReadStorage<'a, InflictsDamage>,
        WriteStorage<'a, CombatStats>,
        WriteStorage<'a, SufferDamage>,
        ReadStorage<'a, AreaOfEffect>,
        WriteStorage<'a, Confusion>,
    );


    fn run(&mut self, data: Self::SystemData) {
        let (mut map, player_entity, mut gamelog, entities, mut wants_use_item, names, consumables, healing, inflict_damage, mut combat_stats, mut suffer_damage, aoe, mut confusion) = data;

        for (entity, useitem) in (&entities, &wants_use_item).join() {
            let mut used_item = true;
            //Targeting
            let mut targets: Vec<Entity> = Vec::new();
            match useitem.target {
                // if no target, apply to player
                None => targets.push(*player_entity),
                Some(target) => {
                    let area_effect = aoe.get(useitem.item);
                    match area_effect {
                        None => {
                            //Single target in tile
                            let idx = map.xy_idx(target.x, target.y);
                            for mob in map.tile_content[idx].iter() {
                                targets.push(*mob);
                            }
                        }
                        Some(area_effect) => {
                            let mut affected_tiles = rltk::field_of_view(target, area_effect.radius, &*map);
                            affected_tiles.retain(|p| p.x > 0 && p.x < map.width - 1 && p.y > 0 && p.y < map.height - 1);
                            for tile_idx in affected_tiles.iter() {
                                let idx = map.xy_idx(tile_idx.x, tile_idx.y);
                                for mob in map.tile_content[idx].iter() {
                                    targets.push(*mob);
                                }
                            }
                        }
                    }
                }
            }


            let item_heals = healing.get(useitem.item);
            match item_heals {
                None => {}
                Some(healer) => {
                    used_item = false;
                    for target in targets.iter() {
                        let stats = combat_stats.get_mut(*target);
                        if let Some(stats) = stats {
                            stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
                            if entity == *player_entity {
                                gamelog.entries.push(format!("You use the {}, healing {} hp.", names.get(useitem.item).unwrap().name, healer.heal_amount));
                            }
                            used_item = true;
                        }
                    }
                }
            }

            // if item deals damage, apply it to target cell
            let item_damages = inflict_damage.get(useitem.item);
            match item_damages {
                None => {}
                Some(damage) => {
                    used_item = false;
                    for mob in targets.iter() {
                        SufferDamage::new_damage(&mut suffer_damage, *mob, damage.damage);
                        if entity == *player_entity && *mob != entity{
                            let mob_name = names.get(*mob).unwrap();
                            let item_name = names.get(useitem.item).unwrap();
                            gamelog.entries.push(format!("You use {} on {}, inflicting {} damage.", item_name.name, mob_name.name, damage.damage));
                        }
                        used_item = true;
                    }

                }
            }

            let mut add_confusion = Vec::new();
            {
                let causes_confusion = confusion.get(useitem.item);
                match causes_confusion {
                    None => {}
                    Some(confusion) => {
                        used_item = false;
                        for mob in targets.iter() {
                            add_confusion.push((*mob, confusion.turns));
                            if entity == *player_entity {
                                let mob_name = names.get(*mob).unwrap();
                                let item_name = names.get(useitem.item).unwrap();
                                gamelog.entries.push(format!("You use {} on {}, confusing them.", item_name.name, mob_name.name));
                            }
                        }
                    }
                }
            }

            for mob in add_confusion.iter() {
                confusion.insert(mob.0, Confusion{turns: mob.1}).expect("Unable to insert status");
            }

            if used_item {
                let consumable = consumables.get(useitem.item);
                match consumable {
                    None => {}
                    Some(consumable) => {
                        entities.delete(useitem.item).expect("Delete failed");
                    }
                }
            }

        }
        wants_use_item.clear();
    }
}


pub struct ItemDropSystem {}

impl<'a> System<'a> for ItemDropSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToDropItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, InBackpack>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, entities, mut wants_drop, names, mut positions, mut backpack) = data;

        for (entity, to_drop) in (&entities, &wants_drop).join() {
            let mut dropper_pos = Position { x: 0, y: 0 };
            {
                let dropped_pos = positions.get(entity).unwrap();
                dropper_pos.x = dropped_pos.x;
                dropper_pos.y = dropped_pos.y;
            }
            positions.insert(to_drop.item, Position { x: dropper_pos.x, y: dropper_pos.y });
            backpack.remove(to_drop.item);

            if entity == *player_entity {
                gamelog.entries.push(format!("You drop the {}.", names.get(to_drop.item).unwrap().name));
            }
        }

        wants_drop.clear();
    }
}