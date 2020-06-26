use super::{
    CombatStats, Consumable, GameLog, InBackpack, Map, Name, Position, ProvidesHealing,
    WantsToDropItem, WantsToPickupItem, WantsToUseItem,
};
use specs::prelude::*;

pub struct ItemCollectionSystem {}

impl<'a> System<'a> for ItemCollectionSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        WriteStorage<'a, WantsToPickupItem>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, InBackpack>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, mut wants_pickup, mut positions, names, mut backpack) =
            data;

        for pickup in wants_pickup.join() {
            positions.remove(pickup.item);
            backpack
                .insert(
                    pickup.item,
                    InBackpack {
                        owner: pickup.collected_by,
                    },
                )
                .expect("Unable to insert backpack entry");

            if pickup.collected_by == *player_entity {
                gamelog.entries.push(format!(
                    "You pick up the {}.",
                    names.get(pickup.item).unwrap().name
                ));
            }
        }

        wants_pickup.clear();
    }
}

pub struct ItemUseSystem {}

impl<'a> System<'a> for ItemUseSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToUseItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Consumable>,
        ReadStorage<'a, ProvidesHealing>,
        WriteStorage<'a, CombatStats>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_entity,
            mut gamelog,
            entities,
            mut wants_use,
            names,
            mut consumables,
            healing,
            mut combat_stats,
        ) = data;

        for (entity, useitem) in (&entities, &wants_use).join() {
            let mut used_item = true;

            // Targeting
            let mut targets = Vec::new();
            match useitem.target {
                None => {
                    targets.push(*player_entity);
                }
                Some(target) => unimplemented!(),
            }

            // If a healing item, heal.
            if let Some(healer) = healing.get(useitem.item) {
                used_item = false;

                for target in targets.iter() {
                    if let Some(stats) = combat_stats.get_mut(*target) {
                        stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
                        if entity == *player_entity {
                            gamelog.entries.push(format!(
                                "You use the {}, healing {} hp.",
                                names.get(useitem.item).unwrap().name,
                                healer.heal_amount
                            ));
                        }
                        used_item = true;
                    }
                }
            }
            if used_item {
                if let Some(consumable) = consumables.get_mut(useitem.item) {
                    if consumable.uses == 1 {
                        entities.delete(useitem.item).expect("Delete failed");
                    } else {
                        consumable.uses -= 1;
                    }
                }
            }
        }

        wants_use.clear();
    }
}

pub struct ItemDropSystem {}

impl<'a> System<'a> for ItemDropSystem {
    type SystemData = (
        ReadExpect<'a, Map>,
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToDropItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, InBackpack>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            map,
            player_entity,
            mut gamelog,
            entities,
            mut wants_drop,
            names,
            mut positions,
            mut backpack,
        ) = data;

        for (entity, to_drop) in (&entities, &wants_drop).join() {
            let mut dropper_pos: Position = Position { x: 0, y: 0 };
            let dropped_pos = positions.get(entity).unwrap();
            dropper_pos.x = dropped_pos.x;
            dropper_pos.y = dropped_pos.y;

            let idx = map.xy_idx(dropper_pos.x, dropper_pos.y);
            if map.tile_content[idx].len() > 1 {
                if entity == *player_entity {
                    gamelog.entries.push(format!(
                        "You can not drop {} here.",
                        names.get(to_drop.item).unwrap().name
                    ));
                }
            } else {
                positions
                    .insert(
                        to_drop.item,
                        Position {
                            x: dropper_pos.x,
                            y: dropper_pos.y,
                        },
                    )
                    .expect("Unable to insert position");
                backpack.remove(to_drop.item);

                if entity == *player_entity {
                    gamelog.entries.push(format!(
                        "You drop the {}.",
                        names.get(to_drop.item).unwrap().name
                    ));
                }
            }
        }

        wants_drop.clear();
    }
}
