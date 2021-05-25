use crate::{CombatStats, Item, RunState, Viewshed, WantsToMelee, WantsToPickUpItem, gamelog::GameLog};
use rltk::{Point, Rltk, VirtualKeyCode, console};
use specs::prelude::*;

use super::{Map, Player, Position, State, TileType};
use std::{
    cmp::{max, min},
    usize,
};

pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();
    let mut viewsheds = ecs.write_storage::<Viewshed>();
    let mut player_pos = ecs.write_resource::<Point>();

    let combat_stats = ecs.read_storage::<CombatStats>();
    let map = ecs.fetch::<Map>();

    let entities = ecs.entities();
    let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();

    for (entity, _player, pos, viewshed) in (&entities, &mut players, &mut positions, &mut viewsheds).join() {
        let dest_x  = pos.x + delta_x;
        let dest_y = pos.y + delta_y;
        if out_of_bounds(dest_x, dest_y, &map) {
            return;
        }
        
        let destination_idx = map.xy_idx(pos.x + delta_x, pos.y + delta_y);

        for potential_target in map.tile_content[destination_idx].iter() {
            let target = combat_stats.get(*potential_target);

            // attaches a WantsToMelee to the attacker
            if let Some(_target) = target {
                wants_to_melee.insert(entity, WantsToMelee{
                    target: *potential_target
                }).expect("Add target failed!");
                return;
            }
        }

        if !map.blocked[destination_idx] {
            pos.x = min(79, max(0, pos.x + delta_x));
            pos.y = min(49, max(0, pos.y + delta_y));

            viewshed.dirty = true;
            player_pos.x = pos.x;
            player_pos.y = pos.y;
        }
    }
}


fn out_of_bounds(dest_x: i32, dest_y: i32, map: &Map) -> bool {
    dest_x < 1 || dest_x > map.width - 1 || dest_y < 1 || dest_y > map.height - 1
}


fn get_item(ecs: &mut World) {
    let player_pos = ecs.fetch::<Point>();
    let player_entity = ecs.fetch::<Entity>();
    let entities = ecs.entities();
    let items = ecs.read_storage::<Item>();
    let positions = ecs.read_storage::<Position>();
    let mut gamelog = ecs.fetch_mut::<GameLog>();

    let mut target_item: Option<Entity> = None;
    for (item_entity, _item, position) in (&entities, &items, &positions).join() {
        if position.x == player_pos.x && position.y == player_pos.y {
            target_item = Some(item_entity);
        }
    }

    match target_item {
        None => gamelog.entries.push("There is nothing here to pick up.".to_string()),
        Some(item) => {
            let mut pickup = ecs.write_storage::<WantsToPickUpItem>();
            pickup.insert(*player_entity, WantsToPickUpItem{
                collected_by: *player_entity,
                item
            }).expect("Unable to insert want to pickup");
        }
    }
}



pub fn player_input(gs: &mut State, ctx: &mut Rltk) -> RunState {
    // handle player movement
    match ctx.key {
        None => return RunState::AwaitingInput, // no key -> Paused State
        Some(key) => match key {

            //Movement
            VirtualKeyCode::Left | VirtualKeyCode::Numpad4 => try_move_player(-1, 0, &mut gs.ecs),
            VirtualKeyCode::Right | VirtualKeyCode::Numpad6 => try_move_player(1, 0, &mut gs.ecs),
            VirtualKeyCode::Up | VirtualKeyCode::Numpad8 => try_move_player(0, -1, &mut gs.ecs),
            VirtualKeyCode::Down | VirtualKeyCode::Numpad2 => try_move_player(0, 1, &mut gs.ecs),

            //Diagonals
            VirtualKeyCode::Numpad9 => try_move_player(1, -1, &mut gs.ecs),
            VirtualKeyCode::Numpad7 => try_move_player(-1, -1, &mut gs.ecs),
            VirtualKeyCode::Numpad1 => try_move_player(-1, 1, &mut gs.ecs),
            VirtualKeyCode::Numpad3 => try_move_player(1, 1, &mut gs.ecs),


            // Item handling
            VirtualKeyCode::G => get_item(&mut gs.ecs),
            VirtualKeyCode::I => return RunState::ShowInventory,
            VirtualKeyCode::D => return RunState::ShowDropItem,

            _ => return RunState::AwaitingInput, //Non-used keys do nothing
        },
    }

    RunState::PlayerTurn
}
