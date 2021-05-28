use specs::saveload::{SimpleMarker, MarkedBuilder, SerializeComponents, DeserializeComponents};
use crate::{SerializeMe, SerializationHelper, Position};
use std::fs;
use std::fs::File;
use std::path::Path;
use specs::{World, WorldExt, Builder};
use specs::error::NoError;
use crate::WantsToPickUpItem;
use crate::*;

// this is hard to understand
macro_rules! serialize_individually {
    ($ecs:expr, $ser:expr, $data:expr, $($type:ty), *) => {
        $(
        SerializeComponents::<NoError, SimpleMarker<SerializeMe>>::serialize(
            &($ecs.read_storage::<$type>(), ),
            &$data.0,
            &$data.1,
            &mut $ser,
        )
        .unwrap();
        )*
    };
}

pub fn save_game(ecs: &mut World) {
    // Create helper
    let mapcopy = ecs.get_mut::<super::map::Map>().unwrap().clone();
    let savehelper = ecs.create_entity().with(SerializationHelper{map: mapcopy}).marked::<SimpleMarker<SerializeMe>>().build();


    // Actual serialization
    {
        let data = (ecs.entities(), ecs.read_storage::<SimpleMarker<SerializeMe>>());

        let writer = File::create("./savegame.json").unwrap();
        let mut serializer = serde_json::Serializer::new(writer);


        // this is kind of messy, but it works
        serialize_individually!(ecs, serializer, data, Position, Renderable, Player, Viewshed, Monster,
            Name, BlocksTile, CombatStats, SufferDamage, WantsToMelee, Item, Consumable, Ranged, InflictsDamage,
            AreaOfEffect, Confusion, ProvidesHealing, InBackpack, WantsToPickUpItem, WantsToUseItem,
            WantsToDropItem, SerializationHelper
        );
    }

    // clean up
    ecs.delete_entity(savehelper).expect("Crash on cleanup");
}

pub fn save_exists() -> bool {
    Path::new("./savegame.json").exists()
}

// macro needed for loading
// is basically the reverse of serialize_individually

macro_rules! deserialize_individually {
    ($ecs: expr, $de: expr, $data: expr, $($type: ty), *) => {
        $(
            DeserializeComponents::<NoError, _>::deserialize(
                &mut (&mut $ecs.write_storage::<$type>(), ),
                &mut $data.0, // entities
                &mut $data.1, // marker
                &mut $data.2, // allocater
                &mut $de,
            )
            .unwrap();
        )*
    };
}

pub fn load_game(ecs: &mut World) {
    { // keep the borrow checker happy
        // delete everything
        let mut to_delete = Vec::new();
        for e in ecs.entities().join() {
            to_delete.push(e);
        }
        for del in to_delete.iter() {
            ecs.delete_entity(*del).expect("Deletion failed");
        }
    }

    let data = fs::read_to_string("./savegame.json").unwrap();
    let mut de = serde_json::Deserializer::from_str(&data);

    {
        let mut d = (&mut ecs.entities(), &mut ecs.write_storage::<SimpleMarker<SerializeMe>>(), &mut ecs.write_resource::<SimpleMarkerAllocator<SerializeMe>>());
        deserialize_individually!(ecs, de, d, Position, Renderable, Player, Viewshed, Monster, 
            Name, BlocksTile, CombatStats, SufferDamage, WantsToMelee, Item, Consumable, Ranged, InflictsDamage, 
            AreaOfEffect, Confusion, ProvidesHealing, InBackpack, WantsToPickUpItem, WantsToUseItem,
            WantsToDropItem, SerializationHelper
        );
    }

    let mut delete_me: Option<Entity> = None;
    { // avoid borrow conflicts
        let entities = ecs.entities();
        let helper = ecs.read_storage::<SerializationHelper>();
        let player = ecs.read_storage::<Player>();
        let position = ecs.read_storage::<Position>();

        for (e, h) in (&entities, &helper).join() {
            let mut worldmap = ecs.write_resource::<super::map::Map>();
            *worldmap = h.map.clone();
            worldmap.tile_content = vec![Vec::new(); super::map::MAPCOUNT];
            delete_me = Some(e);
        }

        for (e, _p, pos) in (&entities, &player, &position).join() {
            let mut ppos = ecs.write_resource::<rltk::Point>();
            *ppos = rltk::Point::new(pos.x, pos.y);
            let mut player_resource = ecs.write_resource::<Entity>();
            *player_resource = e;
        }
    }
    ecs.delete_entity(delete_me.unwrap()).expect("Unable to delete helper");
}

// for permadeath
pub fn delete_save() {
    if Path::new("./savegame.json").exists() {
        std::fs::remove_file("./savegame.json").expect("Error deleting saved game");
    }
}