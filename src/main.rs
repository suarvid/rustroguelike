use gamelog::GameLog;
use inventory_system::{InventorySystem, ItemDropSystem, ItemUseSystem};
use rltk::{GameState, Point, Rltk, VirtualKeyCode, RGB};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};
use serde::*;


mod components;
mod inventory_system;
mod spawner;

pub use components::*;

mod map;

pub use map::*;

mod player;

use player::*;

mod rect;

pub use rect::Rect;

mod visibility_system;

use visibility_system::VisibilitySystem;

mod monster_ai_system;

use monster_ai_system::MonsterAI;

mod map_indexing_system;

use map_indexing_system::MapIndexingSystem;

mod melee_combat_system;

use melee_combat_system::MeleeCombatSystem;

mod damage_system;

use damage_system::DamageSystem;
use crate::gui::MainMenuSelection;

mod gamelog;
mod gui;
mod saveload_system;

pub struct State {
    ecs: World,
}

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
    ShowTargeting {
        range: i32,
        item: Entity,
    },
    MainMenu { menu_selection: gui::MainMenuSelection },
    SaveGame,
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);

        let mut mob = MonsterAI {};
        mob.run_now(&self.ecs);

        let mut mapindex = MapIndexingSystem {};
        mapindex.run_now(&self.ecs);

        let mut melee_comb_system = MeleeCombatSystem {};
        melee_comb_system.run_now(&self.ecs);

        let mut damage_system = DamageSystem {};
        damage_system.run_now(&self.ecs);

        let mut inventory_system = InventorySystem {};
        inventory_system.run_now(&self.ecs);

        let mut item_system = ItemUseSystem {};
        item_system.run_now(&self.ecs);

        let mut drop_items = ItemDropSystem {};
        drop_items.run_now(&self.ecs);

        self.ecs.maintain(); // apply any changes queued up by the systems
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();
        let mut new_runstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            new_runstate = *runstate;
        }



        match new_runstate {
            RunState::MainMenu { .. } => {}
            _ => {
                draw_map(&self.ecs, ctx);

                {
                    damage_system::delete_the_dead(&mut self.ecs);
                    let positions = self.ecs.read_storage::<Position>();
                    let renderables = self.ecs.read_storage::<Renderable>();
                    let map = self.ecs.fetch::<Map>();

                    let mut data = (&positions, &renderables).join().collect::<Vec<_>>();
                    data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
                    for (pos, render) in data.iter() {
                        let idx = map.xy_idx(pos.x, pos.y);
                        if map.visible_tiles[idx] {
                            ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
                        }
                    }

                    gui::draw_ui(&self.ecs, ctx);
                }
            }
        }


        match new_runstate {
            RunState::PreRun => {
                self.run_systems();
                self.ecs.maintain();
                new_runstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                new_runstate = player_input(self, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                self.ecs.maintain();
                new_runstate = RunState::MonsterTurn;
            }
            RunState::MonsterTurn => {
                self.run_systems();
                self.ecs.maintain();
                new_runstate = RunState::AwaitingInput;
            }
            RunState::ShowInventory => {
                let result = gui::show_inventory(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => new_runstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let ranged = self.ecs.read_storage::<Ranged>();
                        let is_item_ranged = ranged.get(item_entity);
                        if let Some(is_item_ranged) = is_item_ranged {
                            new_runstate = RunState::ShowTargeting { range: is_item_ranged.range, item: item_entity };
                        } else {
                            let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                            intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem { item: item_entity, target: None }).expect("Unable to insert intent");
                            new_runstate = RunState::PlayerTurn;
                        }
                    }
                }
            }
            RunState::ShowDropItem => {
                let result = gui::show_drop_item_menu(self, ctx);
                match result.0 {
                    gui::ItemMenuResult::Cancel => new_runstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = result.1.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToDropItem { item: item_entity }).expect("Unable to insert intent");
                        new_runstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::ShowTargeting { range, item } => {
                let result = gui::ranged_target(self, ctx, range);
                match result.0 {
                    gui::ItemMenuResult::Cancel => new_runstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem { item, target: result.1 }).expect("Unable to insert intent");
                        new_runstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::MainMenu{..} => {
                let result = gui::main_menu(self, ctx);
                match result {
                    gui::MainMenuResult::NoSelection{ selected} => new_runstate = RunState::MainMenu{menu_selection: selected},
                    gui::MainMenuResult::Selected{ selected } => {
                        match selected {
                            gui::MainMenuSelection::NewGame => new_runstate = RunState::PreRun,
                            gui::MainMenuSelection::LoadGame => {
                                saveload_system::load_game(&mut self.ecs);
                                new_runstate = RunState::AwaitingInput;
                                saveload_system::delete_save();
                            }
                            gui::MainMenuSelection::Quit => {std::process::exit(0);}
                        }
                    }
                }
            }
            RunState::SaveGame => {
                saveload_system::save_game(&mut self.ecs);

                new_runstate = RunState::MainMenu {menu_selection: gui::MainMenuSelection::LoadGame};
            }
        }
        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = new_runstate;
        }
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;

    let mut context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
        .build()?;
    context.with_post_scanlines(true);

    let mut gs = State {
        ecs: World::new(),
    };

    // Components
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickUpItem>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<InflictsDamage>();
    gs.ecs.register::<AreaOfEffect>();
    gs.ecs.register::<Confusion>();
    gs.ecs.register::<SimpleMarker<SerializeMe>>();
    gs.ecs.register::<SerializationHelper>();
    

    // this has to be inserted before map usage
    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());

    let map: Map = Map::new_map_rooms_and_corridors();
    let (player_x, player_y) = map.rooms[0].center(); //make player spawn in center of "first" room

    let player_entity = spawner::spawn_player(&mut gs.ecs, player_x, player_y);

    // has to be inserted before rooms are spawned
    gs.ecs.insert(rltk::RandomNumberGenerator::new());

    // skip the first room to avoid the player
    // spawning on a mob
    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room);
    }

    gs.ecs.insert(RunState::MainMenu{menu_selection: MainMenuSelection::NewGame});
    gs.ecs.insert(map);
    gs.ecs.insert(Point::new(player_x, player_y));
    gs.ecs.insert(player_entity);
    gs.ecs.insert(gamelog::GameLog {
        entries: vec!["Welcome to Rusty Roguelike".to_string()],
    });
    
    rltk::main_loop(context, gs)
}
