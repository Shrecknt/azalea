//! Simulate the Minecraft world, currently only used for tests.

use std::sync::Arc;

use azalea_client::{
    inventory::InventoryComponent, packet_handling::game::SendPacketEvent, PhysicsState,
};
use azalea_core::{position::Vec3, resource_location::ResourceLocation, tick::GameTick};
use azalea_entity::{
    attributes::AttributeInstance, Attributes, EntityDimensions, Physics, Position,
};
use azalea_world::{ChunkStorage, Instance, InstanceContainer, MinecraftEntityId, PartialInstance};
use bevy_app::App;
use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use uuid::Uuid;

#[derive(Bundle, Clone)]
pub struct SimulatedPlayerBundle {
    pub position: Position,
    pub physics: Physics,
    pub physics_state: PhysicsState,
    pub attributes: Attributes,
    pub inventory: InventoryComponent,
}

impl SimulatedPlayerBundle {
    pub fn new(position: Vec3) -> Self {
        let dimensions = EntityDimensions {
            width: 0.6,
            height: 1.8,
        };

        SimulatedPlayerBundle {
            position: Position::new(position),
            physics: Physics::new(dimensions, &position),
            physics_state: PhysicsState::default(),
            attributes: Attributes {
                speed: AttributeInstance::new(0.1),
                attack_speed: AttributeInstance::new(4.0),
            },
            inventory: InventoryComponent::default(),
        }
    }
}

/// Simulate the Minecraft world to see if certain movements would be possible.
pub struct Simulation {
    pub app: App,
    pub entity: Entity,
    _instance: Arc<RwLock<Instance>>,
}

impl Simulation {
    pub fn new(chunks: ChunkStorage, player: SimulatedPlayerBundle) -> Self {
        let instance_name = ResourceLocation::new("azalea:simulation");

        let instance = Arc::new(RwLock::new(Instance {
            chunks,
            ..Default::default()
        }));

        let mut app = App::new();
        // we don't use all the default azalea plugins because we don't need all of them
        app.add_plugins((
            azalea_physics::PhysicsPlugin,
            azalea_entity::EntityPlugin,
            azalea_client::movement::PlayerMovePlugin,
            super::PathfinderPlugin,
            crate::BotPlugin,
            azalea_client::task_pool::TaskPoolPlugin::default(),
            // for mining
            azalea_client::inventory::InventoryPlugin,
            azalea_client::mining::MinePlugin,
            azalea_client::interact::InteractPlugin,
        ))
        .insert_resource(InstanceContainer {
            instances: [(instance_name.clone(), Arc::downgrade(&instance.clone()))]
                .iter()
                .cloned()
                .collect(),
        })
        .add_event::<SendPacketEvent>();

        app.edit_schedule(bevy_app::Main, |schedule| {
            schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::SingleThreaded);
        });

        let mut entity = app.world.spawn((
            MinecraftEntityId(0),
            azalea_entity::LocalEntity,
            azalea_entity::metadata::PlayerMetadataBundle::default(),
            azalea_entity::EntityBundle::new(
                Uuid::nil(),
                *player.position,
                azalea_registry::EntityKind::Player,
                instance_name,
            ),
            azalea_client::InstanceHolder {
                // partial_instance is never actually used by the pathfinder so
                partial_instance: Arc::new(RwLock::new(PartialInstance::default())),
                instance: instance.clone(),
            },
            InventoryComponent::default(),
        ));
        entity.insert(player);

        let entity_id = entity.id();

        Self {
            app,
            entity: entity_id,
            _instance: instance,
        }
    }
    pub fn tick(&mut self) {
        self.app.world.run_schedule(GameTick);
        self.app.update();
    }
    pub fn position(&self) -> Vec3 {
        **self.app.world.get::<Position>(self.entity).unwrap()
    }
}
