use inventory_system::InventorySystem;
use rltk::{GameState, Point, Rltk, VirtualKeyCode, RGB};
use specs::prelude::*;

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
mod gamelog;
mod gui;

pub struct State {
    ecs: World,
    pub runstate: RunState,
}

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    Paused,
    Running,
    ShowInventory,
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

        self.ecs.maintain(); // apply any changes queued up by the systems
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();
        draw_map(&self.ecs, ctx);
        gui::draw_ui(&self.ecs, ctx);

        match self.runstate {
            RunState::Running => {
                self.run_systems();
                self.runstate = RunState::Paused;
            }
            RunState::Paused => {
                self.runstate = player_input(self, ctx);
            }
            RunState::ShowInventory => {
                if gui::show_inventory(self, ctx) == gui::ItemMenuResult::Cancel {
                    self.runstate = RunState::Paused;
                }
            }
        }

        damage_system::delete_the_dead(&mut self.ecs);
        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();
        let map = self.ecs.fetch::<Map>();

        for (pos, render) in (&positions, &renderables).join() {
            let idx = map.xy_idx(pos.x, pos.y);
            if map.visible_tiles[idx] {
                ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
            }
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
        runstate: RunState::Running,
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
    gs.ecs.register::<Potion>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickUpItem>();

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

    gs.ecs.insert(map);
    gs.ecs.insert(Point::new(player_x, player_y));
    gs.ecs.insert(player_entity);
    gs.ecs.insert(gamelog::GameLog {
        entries: vec!["Welcome to Rusty Roguelike".to_string()],
    });
    rltk::main_loop(context, gs)
}
